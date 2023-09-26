use crate::{player::Player, state::State};
use id3::{Tag, TagLike};
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
	cmp::Ordering,
	collections::VecDeque,
	fmt::Debug,
	fs,
	path::{Path, PathBuf},
	time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
	#[error("couldn't find track {0:?}")]
	NoTrack(PathBuf),
	#[error("queue is empty")]
	NoTracks,
	#[error("out of bounds {0}")]
	OutOfBounds(usize),
}

#[derive(Clone)]
pub struct Track {
	path: PathBuf,
	tag: Tag,
}

impl Serialize for Track {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.path.as_path().serialize(serializer)
	}
}

impl Track {
	// todo don't use
	fn new(path: PathBuf) -> Self {
		assert!(path.exists(), "path {:?} doesn't exist", path);

		let tag = Tag::read_from_path(&path).unwrap_or_default();
		Track { path, tag }
	}

	pub fn maybe_deserialize<'de, D>(data: D) -> Result<Option<Track>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let path_or: Option<PathBuf> = Deserialize::deserialize(data)?;
		let track = path_or.and_then(|path| Track::try_from(path).ok());
		Ok(track)
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

impl Debug for Track {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut dbg = f.debug_struct("Track");
		dbg.field("path", &self.path);

		self.tag.title().map(|title| dbg.field("title", &title));
		self.tag.artist().map(|artist| dbg.field("artist", &artist));
		self.tag.album().map(|album| dbg.field("album", &album));

		dbg.finish()
	}
}

impl TryFrom<PathBuf> for Track {
	type Error = QueueError;
	fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
		if path.exists() {
			let track = Track::new(path);
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

impl Eq for Track {}

impl PartialEq<Track> for Track {
	fn eq(&self, other: &Track) -> bool {
		self.path.eq(&other.path)
	}
}

impl PartialEq<PathBuf> for Track {
	fn eq(&self, other: &PathBuf) -> bool {
		self.path.eq(other)
	}
}

impl Ord for Track {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		let titles = self
			.tag
			.title()
			.zip(other.tag.title())
			.map(|(s, o)| (s.to_lowercase(), o.to_lowercase()));
		let artist = self
			.tag
			.artist()
			.zip(other.tag.artist())
			.map(|(s, o)| (s.to_lowercase(), o.to_lowercase()));
		let albums = self
			.tag
			.album()
			.zip(other.tag.album())
			.map(|(s, o)| (s.to_lowercase(), o.to_lowercase()));

		titles
			.map_or(Ordering::Equal, |(s, o)| s.cmp(&o))
			.then_with(|| artist.map_or(Ordering::Equal, |(s, o)| s.cmp(&o)))
			.then_with(|| albums.map_or(Ordering::Equal, |(s, o)| s.cmp(&o)))
	}
}

impl PartialOrd for Track {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
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
	rng: ThreadRng,
}

impl Queue {
	pub fn state(state: &State) -> Self {
		let (tracks, path) = if let Some(path) = state.queue.as_deref() {
			let mut tracks = Track::directory(path);
			tracks.sort();

			(tracks, Some(path.to_owned()))
		} else {
			(vec![], None)
		};

		let current = state.track.clone();
		let current = current.and_then(|current| {
			let find = tracks.iter().find(|&track| track == &current);
			find.is_some().then_some(current)
		});

		let shuffle = state.shuffle;

		let last = VecDeque::new();
		let next = vec![];

		let rng = rand::thread_rng();

		Queue {
			path,
			tracks,
			last,
			next,
			current,
			shuffle,
			rng,
		}
	}

	#[inline]
	pub fn is_shuffle(&self) -> bool {
		self.shuffle
	}

	pub fn shuffle(&mut self) {
		self.next.clear();
		self.last.clear();

		self.shuffle = !self.shuffle;
	}

	#[inline]
	pub fn path(&self) -> Option<&PathBuf> {
		self.path.as_ref()
	}

	#[inline]
	pub fn tracks(&self) -> &[Track] {
		&self.tracks
	}

	#[inline]
	pub fn current(&self) -> Option<&Track> {
		self.current.as_ref()
	}

	#[inline]
	pub fn idx(&self) -> Option<usize> {
		self.current()
			.and_then(|track| self.tracks().iter().position(|map| track == map))
	}

	#[inline]
	fn get_track(&mut self, idx: usize) -> Result<Track, QueueError> {
		let track = self.tracks.get(idx).ok_or(QueueError::OutOfBounds(idx))?;
		Ok(track.clone())
	}

	// todo last for seq
	pub fn last(&mut self, player: &mut Player) {
		if let Some(track) = self.last.pop_back() {
			player.replace(track.as_str());

			if let Some(current) = self.current.replace(track) {
				self.next.push(current);
			}
		}
	}

	fn nxt_seq(&mut self) -> Result<Track, QueueError> {
		if self.tracks.is_empty() {
			return Err(QueueError::NoTracks);
		}

		let len = self.tracks().len();
		let idx = self.idx();
		let idx = idx.map_or(0, |idx| {
			if idx + 1 >= len {
				0
			} else {
				idx.saturating_add(1)
			}
		});

		self.get_track(idx)
	}

	fn nxt_shuf(&mut self) -> Result<Track, QueueError> {
		let track = if let Some(current) = self.current().cloned() {
			self.tracks
				.iter()
				.filter(|&track| track != &current)
				.choose(&mut self.rng)
		} else {
			self.tracks.iter().choose(&mut self.rng)
		};

		let track = track.ok_or(QueueError::NoTracks)?;
		Ok(track.clone())
	}

	fn nxt(&mut self) -> Result<Track, QueueError> {
		if let Some(track) = self.next.pop() {
			Ok(track)
		} else if self.shuffle {
			self.nxt_shuf()
		} else {
			self.nxt_seq()
		}
	}

	fn replace(&mut self, track: Track, player: &mut Player) {
		player.replace(track.as_str());
		player.pause(false);

		if self.current() != Some(&track) {
			if let Some(current) = self.current.replace(track) {
				self.last.push_back(current);

				// todo this can probably be like a 1000 times higher
				if self.last.len() > 25 {
					self.last.pop_front();
				}
			}
		}
	}

	pub fn next(&mut self, player: &mut Player) -> Result<(), QueueError> {
		let track = self.nxt()?;
		self.replace(track, player);

		Ok(())
	}

	pub fn restart(&self, player: &mut Player) {
		if self.current.is_some() {
			let start = Duration::ZERO;
			player.seek(start);
		}
	}

	pub fn seek_d(&self, player: &mut Player, state: &State, amt: Duration) {
		if self.current.is_some() {
			if let Some(elapsed) = state.elapsed() {
				let position = elapsed.saturating_sub(amt);
				player.seek(position);
			}
		}
	}

	// todo fix loop around at the end
	pub fn seek_i(&mut self, player: &mut Player, state: &State, amt: Duration) {
		if self.current.is_some() {
			if let Some((elapsed, duration)) = state.elapsed_duration() {
				let position = elapsed.saturating_add(amt);

				if position >= duration {
					self.next(player).unwrap();
				} else {
					player.seek(position);
				}
			}
		}
	}

	// todo refactor and shit
	// todo error handling
	pub fn done(&mut self, player: &mut Player, state: &State) {
		if state.almost() {
			let track = self.nxt().unwrap();

			self.current = Some(track.clone());
			player.queue(track.as_str());
		}

		if state.done() {
			self.next(player).unwrap();
		}
	}
}
