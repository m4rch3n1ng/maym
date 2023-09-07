use crate::player::Player;
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};

const PATH: &str = "/home/may/.config/m4rch/player/status.json";

#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
	pub volume: f64,
	#[serde(skip)]
	pub paused: bool,
	pub muted: bool,
	#[serde_as(as = "Option<serde_with::DurationSeconds<u64>>")]
	pub remaining: Option<Duration>,
	#[serde_as(as = "Option<serde_with::DurationSeconds<u64>>")]
	pub duration: Option<Duration>,
	pub track: Option<String>,
}

impl State {
	pub fn init() -> Self {
		let file = fs::read_to_string(PATH).unwrap();
		serde_json::from_str(&file).unwrap()
	}

	pub fn elapsed(&self) -> Option<Duration> {
		self.duration
			.and_then(|duration| self.remaining.map(|remaining| duration - remaining))
	}

	pub fn tick(&mut self, player: &Player) {
		self.volume = player.volume();
		self.paused = player.paused();
		self.muted = player.muted();
		self.remaining = player.remaining();
		self.duration = player.duration();
		self.track = player.track();
	}

	pub fn write(&self) {
		let mut buf = Vec::new();
		let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
		let mut json_serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);

		self.serialize(&mut json_serializer).unwrap();
		let mut serialized = String::from_utf8(buf).unwrap();
		serialized.push('\n');

		fs::write(PATH, serialized).unwrap();
	}
}

impl Default for State {
	fn default() -> Self {
		State {
			volume: 50.0,
			paused: false,
			muted: false,
			remaining: None,
			duration: None,
			track: None,
		}
	}
}
