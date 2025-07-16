//! application [`State`]

#[cfg(feature = "mpris")]
use crate::mpris::{Mpris, MprisUpdate};
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
	path::PathBuf,
	sync::LazyLock,
	time::Duration,
};
use thiserror::Error;

#[cfg(not(feature = "mpris"))]
type Mpris = ();

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

/// struct to track application state
///
/// also used to reinstate on startup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
	/// volume
	pub volume: u8,
	/// is paused
	#[serde(skip, default = "_default_true")]
	pub paused: bool,
	/// is muted
	pub muted: bool,
	/// track time elapsed
	#[serde(with = "duration")]
	elapsed: Option<Duration>,
	/// track time length
	#[serde(with = "duration")]
	duration: Option<Duration>,
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
		self.elapsed
	}

	/// track duration
	#[inline]
	pub fn duration(&self) -> Option<Duration> {
		self.duration
	}

	/// update self to reflect current application state
	pub fn tick(&mut self, player: &mut Player, queue: &Queue, ui: &mut Ui, mpris: &mut Mpris) {
		#[cfg(not(feature = "mpris"))]
		let _ = mpris;

		player.update();

		let volume = player.volume();
		if self.volume != volume {
			self.volume = volume;
			#[cfg(feature = "mpris")]
			mpris.update(MprisUpdate::Volume);
		}

		let paused = player.paused();
		if self.paused != paused {
			self.paused = paused;
			#[cfg(feature = "mpris")]
			mpris.update(MprisUpdate::PlayerStatus);
		}

		let muted = player.muted();
		if self.muted != muted {
			self.muted = muted;
			#[cfg(feature = "mpris")]
			mpris.update(MprisUpdate::Volume);
		}

		self.duration = player.duration();
		self.elapsed = player.elapsed();

		let shuffle = queue.is_shuffle();
		if self.shuffle != shuffle {
			self.shuffle = shuffle;
			#[cfg(feature = "mpris")]
			mpris.update(MprisUpdate::Shuffle);
		}

		let q = queue.path();
		if self.queue.as_deref() != q {
			ui.change_queue(queue);
			self.queue = q.map(ToOwned::to_owned);
		}

		if self.track.as_ref() != queue.track() {
			ui.change_track(queue);
			self.track = queue.track().cloned();
			#[cfg(feature = "mpris")]
			mpris.update(MprisUpdate::Metadata);
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

mod duration {
	use serde::{Deserialize, Serialize};
	use std::time::Duration;

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let secs = Option::<u64>::deserialize(deserializer)?;
		let duration = secs.map(Duration::from_secs);
		Ok(duration)
	}

	pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let secs = value.map(|duration| duration.as_secs());
		secs.serialize(serializer)
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
