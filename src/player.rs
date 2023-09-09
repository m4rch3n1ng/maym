use crate::queue::Queue;
use crate::state::State;
use mpv::{MpvHandler, MpvHandlerBuilder};
use std::fmt::Debug;
use std::time::Duration;

pub struct Player(MpvHandler);

impl Debug for Player {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Player")
	}
}

impl Default for Player {
	fn default() -> Self {
		Player::new()
	}
}

impl Player {
	pub fn new() -> Self {
		let mut mpv = MpvHandlerBuilder::new()
			.expect("couldn't init mpv handler builder")
			.build()
			.expect("couldn't build mpv handler builder");

		mpv.set_option("vo", "null").expect("couldn't set vo=null");

		Player(mpv)
	}

	pub fn state(&mut self, queue: &Queue, state: &State) {
		self.0.set_property("volume", state.volume as i64).unwrap();
		self.0.set_property("mute", state.muted).unwrap();

		if let Some(track) = queue.track() {
			let start = state.elapsed();
			let start = start.unwrap_or_default();

			let track = track.as_str();
			self.restart(track, start);
		}
	}

	pub fn queue(&mut self, track: &str) {
		self.0
			.command(&["loadfile", &track, "append-play"])
			.expect("error loading file");
	}

	pub fn seek(&mut self, start: Duration) {
		let start = start.as_secs_f64();
		self.0.set_property("time-pos", start).unwrap();
	}

	fn restart(&mut self, track: &str, start: Duration) {
		let start = format!("start={},pause=yes", start.as_secs());
		self.0
			.command(&["loadfile", track, "replace", &start])
			.expect("couldn't reload file");
	}

	pub fn replace(&mut self, track: &str) {
		self.0
			.command(&["loadfile", track, "replace"])
			.expect("error loading file");
	}

	pub fn toggle(&mut self) {
		let paused = self.paused();
		self.0
			.set_property("pause", !paused)
			.expect("couldn't toggle player");
	}

	// todo do smth with negative values
	pub fn volume(&self) -> u64 {
		let vol = self
			.0
			.get_property::<i64>("volume")
			.expect("couldn't get volume");
		vol as u64
	}

	pub fn paused(&self) -> bool {
		self.0
			.get_property("pause")
			.expect("couldn't get pause state")
	}

	pub fn duration(&self) -> Option<Duration> {
		match self.0.get_property("duration") {
			Ok(duration) => Some(Duration::from_secs_f64(duration)),
			Err(mpv::Error::MPV_ERROR_PROPERTY_UNAVAILABLE) => None,
			Err(err) => panic!("couldn't get duration {}", err),
		}
	}

	pub fn remaining(&self) -> Option<Duration> {
		match self.0.get_property("time-remaining") {
			Ok(remaining) => Some(Duration::from_secs_f64(remaining)),
			Err(mpv::Error::MPV_ERROR_PROPERTY_UNAVAILABLE) => None,
			Err(err) => panic!("couldn't get duration {}", err),
		}
	}

	pub fn mute(&mut self) {
		let muted = self.muted();
		self.0
			.set_property("mute", !muted)
			.expect("couldn't set mute")
	}

	pub fn muted(&self) -> bool {
		self.0.get_property("mute").expect("couldn't get mute")
	}

	pub fn i_vol(&mut self, amt: u64) {
		let vol = self.volume();
		let vol = u64::min(100, vol + amt);

		self.0
			.set_property("volume", vol as i64)
			.expect("couldn't get volume");
	}

	pub fn d_vol(&mut self, amt: u64) {
		let vol = self.volume();
		let vol = vol.saturating_sub(amt);

		self.0
			.set_property("volume", vol as i64)
			.expect("couldn't set volume");
	}
}
