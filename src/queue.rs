use rand::seq::IteratorRandom;
use std::{
	fs,
	path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{player::Player, state::State};

#[derive(Debug, Error)]
pub enum QueueError {
	#[error("couldn't find track {0:?}")]
	NoTrack(PathBuf),
	#[error("queue is empty")]
	NoTracks,
}

#[derive(Debug, Clone)]
pub struct Track {
	path: PathBuf,
}

impl AsRef<str> for Track {
	fn as_ref(&self) -> &str {
		self.path.to_str().unwrap()
	}
}

impl Track {
	// todo don't use
	pub fn new(path: PathBuf) -> Self {
		assert!(path.exists(), "path {:?} doesn't exist", path);
		Track { path }
	}

	pub fn directory<P: AsRef<Path>>(path: P) -> Vec<Self> {
		let path = path.as_ref();
		assert!(path.is_dir(), "path {:?} is not a directiry", path);

		let files = fs::read_dir(path).unwrap();
		files
			.into_iter()
			.flatten()
			.map(|entry| entry.path())
			.map(Track::new)
			.collect::<Vec<_>>()
	}

	pub fn path_str(&self) -> String {
		self.path
			.clone()
			.into_os_string()
			.into_string()
			.unwrap_or_else(|path| panic!("path {:?} is not utf-8", path))
	}
}

impl TryFrom<PathBuf> for Track {
	type Error = QueueError;
	fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
		if path.exists() {
			let track = Track { path };
			Ok(track)
		} else {
			Err(QueueError::NoTrack(path))
		}
	}
}

impl TryFrom<&str> for Track {
	type Error = QueueError;
	fn try_from(string: &str) -> Result<Self, Self::Error> {
		let path = PathBuf::from(string);
		Track::try_from(path)
	}
}

impl PartialEq<PathBuf> for Track {
	fn eq(&self, other: &PathBuf) -> bool {
		self.path.eq(other)
	}
}

#[derive(Debug)]
pub struct Queue {
	path: Option<PathBuf>,
	tracks: Vec<Track>,
	current: Option<Track>,
	shuffle: bool,
}

impl Queue {
	pub fn state(state: &State) -> Self {
		let (tracks, path) = if let Some(path) = state.queue.as_ref() {
			(Track::directory(path), Some(path.clone()))
		} else {
			(vec![], None)
		};

		let current = state
			.track
			.as_deref()
			.and_then(|track| Track::try_from(track).ok());
		let shuffle = state.shuffle;

		Queue {
			path,
			tracks,
			current,
			shuffle,
		}
	}

	pub fn is_shuffle(&self) -> bool {
		self.shuffle
	}

	pub fn path(&self) -> Option<PathBuf> {
		self.path.clone()
	}

	pub fn track(&self) -> Option<&Track> {
		self.current.as_ref()
	}

	pub fn next(&mut self, player: &mut Player) -> Result<(), QueueError> {
		let mut rng = rand::thread_rng();
		let track = self
			.tracks
			.iter()
			.choose(&mut rng)
			.ok_or(QueueError::NoTracks)?;

		self.current = Some(track.clone());

		let track_str = track.path_str();
		player.replace(&track_str);

		Ok(())
	}
}
