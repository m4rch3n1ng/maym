use rand::seq::IteratorRandom;
use std::{
	collections::VecDeque,
	fs,
	path::{Path, PathBuf},
	time::Duration,
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

	pub fn as_str(&self) -> &str {
		self.path
			.to_str()
			.unwrap_or_else(|| panic!("invalid utf-8 in {:?}", self.path))
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
	last: VecDeque<Track>,
	next: Vec<Track>,
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

		let last = VecDeque::new();
		let next = vec![];

		Queue {
			path,
			tracks,
			last,
			next,
			current,
			shuffle,
		}
	}

	#[inline]
	pub fn is_shuffle(&self) -> bool {
		self.shuffle
	}

	#[inline]
	pub fn path(&self) -> Option<PathBuf> {
		self.path.clone()
	}

	#[inline]
	pub fn track(&self) -> Option<&Track> {
		self.current.as_ref()
	}

	fn dequeue(&mut self) {
		if let Some(current) = self.current.as_ref() {
			self.last.push_back(current.clone());

			if self.last.len() > 25 {
				self.last.pop_front();
			}
		}
	}

	pub fn last(&mut self, player: &mut Player) {
		if let Some(current) = self.current.as_ref() {
			self.next.push(current.clone());
		}

		if let Some(track) = self.last.pop_back() {
			player.replace(track.as_str());
			self.current = Some(track);
		}
	}

	pub fn next(&mut self, player: &mut Player) -> Result<(), QueueError> {
		self.dequeue();

		if let Some(track) = self.next.pop() {
			player.replace(track.as_str());
			self.current = Some(track);
		} else {
			let mut rng = rand::thread_rng();
			let track = self
				.tracks
				.iter()
				.choose(&mut rng)
				.ok_or(QueueError::NoTracks)?;

			self.current = Some(track.clone());
			player.replace(track.as_str());
		}

		Ok(())
	}

	pub fn restart(&self, player: &mut Player) {
		if self.current.is_some() {
			let start = Duration::ZERO;
			player.seek(start);
		}
	}

	pub fn seek_d(&self, player: &mut Player, state: &State, amt: u64) {
		if self.current.is_some() {
			if let Some(elapsed) = state.elapsed() {
				let amt = Duration::from_secs(amt);
				let start = elapsed.saturating_sub(amt);

				player.seek(start)
			}
		}
	}

	// todo fix loop around at the end
	pub fn seek_i(&self, player: &mut Player, state: &State, amt: u64) {
		if self.current.is_some() {
			if let Some(elapsed) = state.elapsed() {
				let duration = state
					.duration
					.expect("state.duration should unwrap if state.elapsed() is some");

				let amt = Duration::from_secs(amt);
				let start = Duration::min(duration, elapsed + amt);

				player.seek(start);
			}
		}
	}

	// todo refactor and shit
	// todo error handling
	pub fn done(&mut self, player: &mut Player, state: &State) {
		if state.almost() {
			let mut rng = rand::thread_rng();
			let track = self
				.tracks
				.iter()
				.choose(&mut rng)
				.ok_or(QueueError::NoTracks)
				.unwrap();

			self.current = Some(track.clone());
			player.queue(track.as_str());
		}

		if state.done() {
			self.next(player).unwrap();
		}
	}
}
