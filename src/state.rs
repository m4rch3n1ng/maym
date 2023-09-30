use crate::{
	player::Player,
	queue::{Queue, Track},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::Duration};
use thiserror::Error;

const PATH: &str = "/home/may/.config/m4rch/player/status.json";

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum StateError {
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("serde error")]
	SerdeJsonError(#[from] serde_json::Error),
	#[error("from utf8 error")]
	FromUtf8Error(#[from] std::string::FromUtf8Error),
}

const fn _default_true() -> bool {
	true
}

#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
	pub volume: u64,
	#[serde(skip, default = "_default_true")]
	pub paused: bool,
	pub muted: bool,
	#[serde_as(as = "Option<serde_with::DurationSeconds>")]
	pub elapsed: Option<Duration>,
	#[serde_as(as = "Option<serde_with::DurationSeconds>")]
	pub duration: Option<Duration>,
	pub shuffle: bool,
	pub queue: Option<PathBuf>,
	#[serde(deserialize_with = "Track::maybe_deserialize")]
	pub track: Option<Track>,
}

impl State {
	pub fn init() -> Self {
		fs::read_to_string(PATH)
			.ok()
			.and_then(|file| serde_json::from_str(&file).ok())
			.unwrap_or_default()
	}

	pub fn elapsed_duration(&self) -> Option<(Duration, Duration)> {
		self.elapsed.zip(self.duration)
	}

	#[inline]
	pub fn elapsed(&self) -> Option<Duration> {
		self.elapsed
	}

	pub fn remaining(&self) -> Option<Duration> {
		self.duration
			.and_then(|duration| self.elapsed.map(|elapsed| duration.saturating_sub(elapsed)))
	}

	#[inline]
	pub fn done(&self) -> bool {
		!self.paused && self.track.is_some() && self.duration.is_none() && self.elapsed.is_none()
	}

	pub fn almost(&self) -> bool {
		if self.paused {
			return false;
		}

		let threshold = Duration::from_millis(500);

		if let Some(remaining) = self.remaining() {
			remaining <= threshold
		} else {
			false
		}
	}

	pub fn tick(&mut self, player: &Player, queue: &Queue) {
		self.volume = player.volume();
		self.paused = player.paused();
		self.muted = player.muted();
		self.duration = player.duration();
		self.elapsed = player.elapsed();

		self.shuffle = queue.is_shuffle();
		self.queue = queue.path().cloned();

		if self.track.as_ref() != queue.current() {
			self.track = queue.current().cloned();
		}
	}

	pub fn write(&self) -> Result<(), StateError> {
		let mut buf = Vec::new();
		let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
		let mut json_serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);

		self.serialize(&mut json_serializer)?;
		let mut serialized = String::from_utf8(buf)?;
		serialized.push('\n');

		fs::write(PATH, serialized)?;

		Ok(())
	}
}

impl Default for State {
	fn default() -> Self {
		State {
			volume: 50,
			paused: true,
			muted: false,
			elapsed: None,
			duration: None,
			shuffle: true,
			queue: None,
			track: None,
		}
	}
}
