use crate::{
	player::Player,
	queue::{Queue, Track},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::Duration};

const PATH: &str = "/home/may/.config/m4rch/player/status.json";

#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
	pub volume: u64,
	#[serde(skip)]
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
		let file = fs::read_to_string(PATH).unwrap();
		serde_json::from_str(&file).unwrap()
	}

	pub fn elapsed_duration(&self) -> Option<(Duration, Duration)> {
		if let Some(elapsed) = self.elapsed {
			self.duration.map(|duration| (elapsed, duration))
		} else {
			None
		}
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
		self.queue = queue.path();

		if self.track.as_ref() != queue.track() {
			self.track = queue.track().cloned()
		}
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
			volume: 50,
			paused: false,
			muted: false,
			elapsed: None,
			duration: None,
			shuffle: true,
			queue: None,
			track: None,
		}
	}
}
