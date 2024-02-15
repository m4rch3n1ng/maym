//! application [`State`]

use crate::{
	config::CONFIG_DIR,
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
	path::PathBuf,
	time::Duration,
};
use thiserror::Error;

/// path for state file
static STATE_PATH: Lazy<PathBuf> = Lazy::new(|| CONFIG_DIR.join("status.json"));

/// state error
#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum StateError {
	/// io error
	#[error("io error")]
	IoError(#[from] std::io::Error),
	/// serde error
	#[error("serde error")]
	SerdeJsonError(#[from] serde_json::Error),
}

/// const eval to true, used for #[serde(default = "...")]
const fn _default_true() -> bool {
	true
}

/// struct to track application state
///
/// also used to reinstate on startup
#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
	/// volume
	pub volume: u64,
	/// is paused
	#[serde(skip, default = "_default_true")]
	pub paused: bool,
	/// is muted
	pub muted: bool,
	#[serde_as(as = "Option<serde_with::DurationSeconds>")]
	/// track time elapsed
	pub elapsed: Option<Duration>,
	#[serde_as(as = "Option<serde_with::DurationSeconds>")]
	/// track time length
	pub duration: Option<Duration>,
	/// [`Queue`] is shuffle
	pub shuffle: bool,
	/// [`Utf8PathBuf`] to queue
	pub queue: Option<Utf8PathBuf>,
	/// current [`Track`]
	#[serde(deserialize_with = "Track::maybe_deserialize")]
	pub track: Option<Track>,
}

impl State {
	/// read from file and use [`Default::default`] on error
	pub fn init() -> Self {
		fs::read_to_string(&*STATE_PATH)
			.ok()
			.and_then(|file| serde_json::from_str(&file).ok())
			.unwrap_or_default()
	}

	/// time elapsed and duration
	#[inline]
	pub fn elapsed_duration(&self) -> Option<(Duration, Duration)> {
		self.elapsed.zip(self.duration)
	}

	/// elapsed time
	#[inline]
	pub fn elapsed(&self) -> Option<Duration> {
		self.elapsed
	}

	/// return `true` if track is done
	///
	/// returns `true` if
	/// - isn't paused
	/// - a track is playing
	/// - the duration and elapsed don't exist
	#[inline]
	pub fn done(&self) -> bool {
		!self.paused && self.track.is_some() && self.duration.is_none() && self.elapsed.is_none()
	}

	/// update self to reflect current application state
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

	/// write to file
	pub fn write(&self) -> Result<(), StateError> {
		let file = if let Ok(file) = File::create(&*STATE_PATH) {
			file
		} else {
			fs::create_dir_all(&*CONFIG_DIR)?;
			File::create(&*STATE_PATH)?
		};
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
