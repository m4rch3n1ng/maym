use mpv::{MpvHandler, MpvHandlerBuilder};
use std::fmt::Debug;

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
		mpv.set_option("volume", "50").expect("couldn't set volume");

		Player(mpv)
	}

	pub fn queue(&mut self, track: &str) {
		self.0
			.command(&["loadfile", &track, "append-play"])
			.expect("error loading file");
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

	pub fn volume(&self) -> f64 {
		self.0.get_property("volume").expect("couldn't get volume")
	}

	pub fn paused(&self) -> bool {
		self.0
			.get_property("pause")
			.expect("couldn't get pause state")
	}

	pub fn duration(&self) -> Option<f64> {
		match self.0.get_property("duration") {
			Ok(duration) => Some(duration),
			Err(mpv::Error::MPV_ERROR_PROPERTY_UNAVAILABLE) => None,
			Err(err) => panic!("couldn't get duration {}", err),
		}
	}

	pub fn remaining(&self) -> Option<f64> {
		match self.0.get_property("time-remaining") {
			Ok(remaining) => Some(remaining),
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

	pub fn i_vol(&mut self, amt: f64) {
		let vol = self.volume();
		let vol = f64::min(100f64, vol + amt);

		self.0
			.set_property("volume", vol)
			.expect("couldn't get volume");
	}

	pub fn d_vol(&mut self, amt: f64) {
		let vol = self.volume();
		let vol = f64::max(0f64, vol - amt);

		self.0
			.set_property("volume", vol)
			.expect("couldn't set volume");
	}
}
