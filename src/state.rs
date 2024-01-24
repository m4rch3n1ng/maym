use crate::{
	player::Player,
	queue::{Queue, Track},
	ui::Ui,
};
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
	fs::{self, File},
	io::{BufWriter, Write},
	time::Duration,
};
use thiserror::Error;

static PATH: Lazy<Utf8PathBuf> =
	Lazy::new(|| Utf8PathBuf::from("/home/may/.config/m4rch/player/status.json"));

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum StateError {
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("serde error")]
	SerdeJsonError(#[from] serde_json::Error),
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
	pub queue: Option<Utf8PathBuf>,
	#[serde(deserialize_with = "Track::maybe_deserialize")]
	pub track: Option<Track>,
}

impl State {
	pub fn init() -> Self {
		fs::read_to_string(&*PATH)
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

	#[inline]
	pub fn done(&self) -> bool {
		!self.paused && self.track.is_some() && self.duration.is_none() && self.elapsed.is_none()
	}

	pub fn tick(&mut self, player: &Player, queue: &Queue, ui: &mut Ui) {
		self.volume = player.volume();
		self.paused = player.paused();
		self.muted = player.muted();
		self.duration = player.duration();
		self.elapsed = player.elapsed();

		self.shuffle = queue.is_shuffle();

		let q = queue.path();
		if self.queue.as_ref() != q {
			ui.reset_q(queue);
			self.queue = q.cloned();
		}

		if self.track.as_ref() != queue.track() {
			ui.reset(queue);
			self.track = queue.track().cloned();
		}
	}

	pub fn write(&self) -> Result<(), StateError> {
		let file = File::create(&*PATH)?;
		let mut file = BufWriter::new(file);

		let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
		let mut json_serializer = serde_json::Serializer::with_formatter(&mut file, formatter);

		self.serialize(&mut json_serializer)?;
		writeln!(file)?;

		file.flush()?;
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
