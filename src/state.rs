//! application [`State`]

use crate::{
	config::CONFIG_DIR,
	player::Player,
	queue::{Queue, Track},
	ui::Ui,
};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::{
	fs::{self, File},
	io::{BufWriter, Write},
	ops::{Deref, DerefMut},
	path::PathBuf,
	sync::LazyLock,
	time::Duration,
};
use thiserror::Error;

/// path for state file
static STATE_PATH: LazyLock<PathBuf> = LazyLock::new(|| CONFIG_DIR.join("status.json"));

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

#[derive(Debug)]
struct DurationWrap(Duration);

impl Deref for DurationWrap {
	type Target = Duration;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for DurationWrap {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<'de> Deserialize<'de> for DurationWrap {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let secs = u64::deserialize(deserializer)?;
		let duration = Duration::from_secs(secs);
		Ok(DurationWrap(duration))
	}
}

impl Serialize for DurationWrap {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let secs = self.0.as_secs();
		secs.serialize(serializer)
	}
}

/// struct to track application state
///
/// also used to reinstate on startup
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
	/// volume
	pub volume: u8,
	/// is paused
	#[serde(skip, default = "_default_true")]
	pub paused: bool,
	/// is muted
	pub muted: bool,
	/// track time elapsed
	elapsed: Option<DurationWrap>,
	/// track time length
	duration: Option<DurationWrap>,
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
		self.elapsed().zip(self.duration())
	}

	/// elapsed time
	#[inline]
	pub fn elapsed(&self) -> Option<Duration> {
		self.elapsed.as_deref().copied()
	}

	/// track duration
	#[inline]
	pub fn duration(&self) -> Option<Duration> {
		self.duration.as_deref().copied()
	}

	/// update self to reflect current application state
	pub fn tick(&mut self, player: &mut Player, queue: &Queue, ui: &mut Ui) {
		player.update();

		self.volume = player.volume();
		self.paused = player.paused();
		self.muted = player.muted();
		self.duration = player.duration().map(DurationWrap);
		self.elapsed = player.elapsed().map(DurationWrap);

		self.shuffle = queue.is_shuffle();

		let q = queue.path();
		if self.queue.as_deref() != q {
			ui.reset_q(queue);
			self.queue = q.map(ToOwned::to_owned);
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

#[cfg(test)]
pub mod test {
	use super::State;
	use crate::queue::{QueueError, Track};
	use camino::Utf8PathBuf;

	pub fn mock<P: Into<Utf8PathBuf>>(
		queue: Option<P>,
		track: Option<P>,
	) -> Result<State, QueueError> {
		let queue = queue.map(Into::into);
		let track = track.map(Into::into).map(Track::new).transpose()?;

		let state = State {
			volume: 45,
			paused: true,
			muted: false,
			elapsed: None,
			duration: None,
			queue,
			shuffle: true,
			track,
		};
		Ok(state)
	}
}
