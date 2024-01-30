use crate::queue::Queue;
use crate::state::State;
use conv::{ConvUtil, UnwrapOrSaturate};
use libmpv::{FileState, Mpv};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MpvError {
	#[error("success")]
	Success,
	#[error("event queue full")]
	EventQueueFull,
	#[error("memory allocation failed")]
	NoMem,
	#[error("the mpv core wasn't configured and initialized yet")]
	Uninitialized,
	#[error("invalid parameter")]
	InvalidParameter,
	#[error("option doesn't exist")]
	OptionNotFound,
	#[error("unsupported mpv option format")]
	OptionFormat,
	#[error("setting the option failed")]
	OptionError,
	#[error("property not found")]
	PropertyNotFound,
	#[error("unsupported property format")]
	PropertyFormat,
	#[error("property exists but is unavailable")]
	PropertyUnvailable,
	#[error("error setting or getting poperty")]
	PropertyError,
	#[error("error running command")]
	Command,
	#[error("error loading")]
	LoadingFailed,
	#[error("initializing audio output failed")]
	AoInitFailed,
	#[error("initializing video output failed")]
	VoInitFailed,
	#[error("no audio or video data to play")]
	NothingToPlay,
	#[error("unknown file format")]
	UnknownFormat,
	#[error("certain system requirements are not fulfilled")]
	Unsupported,
	#[error("api function not implemented")]
	NotImplemented,
	#[error("unspecified error")]
	Generic,
	#[error("unknown mpv error")]
	Unknown,
}

impl From<i32> for MpvError {
	fn from(value: i32) -> Self {
		match value {
			0..=i32::MAX => MpvError::Success,
			-1 => MpvError::EventQueueFull,
			-2 => MpvError::NoMem,
			-3 => MpvError::Uninitialized,
			-4 => MpvError::InvalidParameter,
			-5 => MpvError::OptionNotFound,
			-6 => MpvError::OptionFormat,
			-7 => MpvError::OptionError,
			-8 => MpvError::PropertyNotFound,
			-9 => MpvError::PropertyFormat,
			-10 => MpvError::PropertyUnvailable,
			-11 => MpvError::PropertyError,
			-12 => MpvError::Command,
			-13 => MpvError::LoadingFailed,
			-14 => MpvError::AoInitFailed,
			-15 => MpvError::VoInitFailed,
			-16 => MpvError::NothingToPlay,
			-17 => MpvError::UnknownFormat,
			-18 => MpvError::Unsupported,
			-19 => MpvError::NotImplemented,
			-20 => MpvError::Generic,
			i32::MIN..=-21 => MpvError::Unknown,
		}
	}
}

#[derive(Debug, Error)]
pub enum PlayerError {
	#[error("error loading file {index}")]
	LoadFiles {
		index: usize,
		#[source]
		error: Box<PlayerError>,
	},
	#[error("version mismatch")]
	VersionMismatch { linked: u64, loaded: u64 },
	#[error("invalid utf8")]
	InvalidUtf8,
	#[error("null error")]
	Null,
	#[error("mpv error")]
	MpvError(#[source] MpvError),
}

impl From<libmpv::Error> for PlayerError {
	fn from(value: libmpv::Error) -> Self {
		match value {
			libmpv::Error::Loadfiles { index, error } => match Rc::into_inner(error) {
				Some(error) => PlayerError::LoadFiles {
					index,
					error: Box::new(PlayerError::from(error)),
				},
				None => unreachable!(),
			},
			libmpv::Error::VersionMismatch { linked, loaded } => {
				PlayerError::VersionMismatch { linked, loaded }
			}
			libmpv::Error::InvalidUtf8 => PlayerError::InvalidUtf8,
			libmpv::Error::Null => PlayerError::Null,
			libmpv::Error::Raw(err) => {
				let mpv_err = MpvError::from(err);
				PlayerError::MpvError(mpv_err)
			}
		}
	}
}

/// wrapper struct around [`libmpv::Mpv`]
pub struct Player(Mpv);

impl Debug for Player {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Player")
	}
}

impl Player {
	pub fn new() -> color_eyre::Result<Self> {
		let mpv = Mpv::new().map_err(PlayerError::from)?;

		mpv.set_property("vo", "null").map_err(PlayerError::from)?;

		let player = Player(mpv);
		Ok(player)
	}

	pub fn state(&mut self, queue: &Queue, state: &State) -> color_eyre::Result<()> {
		self.0
			.set_property("volume", state.volume as i64)
			.map_err(PlayerError::from)?;
		self.0
			.set_property("mute", state.muted)
			.map_err(PlayerError::from)?;

		if let Some(track) = queue.track() {
			let start = state.elapsed();
			let start = start.unwrap_or_default();

			let track = track.as_str();
			self.revive(track, start)?;
		}

		Ok(())
	}

	fn revive(&mut self, track: &str, start: Duration) -> Result<(), PlayerError> {
		let start = format!("start={},pause=yes", start.as_secs());
		let file = (track, FileState::Replace, Some::<&str>(&start));
		self.0.playlist_load_files(&[file])?;

		Ok(())
	}

	pub fn replace(&mut self, track: &str) {
		self.0
			.playlist_load_files(&[(track, FileState::Replace, None)])
			.map_err(PlayerError::from)
			.expect("error loading file");
	}

	pub fn seek(&mut self, position: Duration) {
		let start = position.as_secs_f64();
		self.0
			.set_property("time-pos", start)
			.map_err(PlayerError::from)
			.expect("couldn't set time-pos");
	}

	pub fn toggle(&mut self) {
		let paused = self.paused();
		self.0
			.set_property("pause", !paused)
			.map_err(PlayerError::from)
			.expect("couldn't toggle player");
	}

	pub fn pause(&mut self, value: bool) {
		self.0
			.set_property("pause", value)
			.map_err(PlayerError::from)
			.expect("couldn't pause player");
	}

	pub fn volume(&self) -> u64 {
		let vol = self
			.0
			.get_property::<i64>("volume")
			.map_err(PlayerError::from)
			.expect("couldn't get volume");
		vol.approx_as::<u64>().unwrap_or_saturate()
	}

	pub fn paused(&self) -> bool {
		self.0
			.get_property("pause")
			.map_err(PlayerError::from)
			.expect("couldn't get pause state")
	}

	pub fn duration(&self) -> Option<Duration> {
		match self.0.get_property("duration").map_err(PlayerError::from) {
			Ok(duration) => Some(Duration::from_secs_f64(duration)),
			Err(PlayerError::MpvError(MpvError::PropertyUnvailable)) => None,
			Err(err) => panic!("couldn't get duration {}", err),
		}
	}

	pub fn elapsed(&self) -> Option<Duration> {
		match self.0.get_property("time-pos").map_err(PlayerError::from) {
			Ok(elapsed) => {
				let elapsed = f64::max(0.0, elapsed);
				Some(Duration::from_secs_f64(elapsed))
			}
			Err(PlayerError::MpvError(MpvError::PropertyUnvailable)) => None,
			Err(err) => panic!("couldn't get duration {}", err),
		}
	}

	pub fn mute(&mut self) {
		let muted = self.muted();
		self.0
			.set_property("mute", !muted)
			.map_err(PlayerError::from)
			.expect("couldn't set mute");
	}

	pub fn muted(&self) -> bool {
		self.0
			.get_property("mute")
			.map_err(PlayerError::from)
			.expect("couldn't get mute")
	}

	pub fn i_vol(&mut self, amt: u64) {
		let vol = self.volume();
		let vol = u64::min(100, vol.saturating_add(amt));

		self.0
			.set_property("volume", vol as i64)
			.map_err(PlayerError::from)
			.expect("couldn't get volume");
	}

	pub fn d_vol(&mut self, amt: u64) {
		let vol = self.volume();
		let vol = vol.saturating_sub(amt);

		self.0
			.set_property("volume", vol as i64)
			.map_err(PlayerError::from)
			.expect("couldn't set volume");
	}
}
