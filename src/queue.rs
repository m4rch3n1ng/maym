use crate::{player::Player, state::State, ui::utils};
use camino::{Utf8Path, Utf8PathBuf};
use id3::{Tag, TagLike};
use itertools::Itertools;
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use ratatui::{style::Stylize, text::Line};
use serde::{Deserialize, Deserializer, Serialize};
use std::{cmp::Ordering, collections::VecDeque, fmt::Debug, fmt::Display, fs, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
	#[error("couldn't find track {0:?}")]
	NoTrack(Utf8PathBuf),
	#[error("queue is empty")]
	NoTracks,
	#[error("out of bounds {0}")]
	OutOfBounds(usize),
	#[error("not a directory {0:?}")]
	NotADirectory(Utf8PathBuf),
	#[error("io error")]
	IoError(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct Track {
	pub path: Utf8PathBuf,
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
	fn new(path: Utf8PathBuf) -> Result<Self, QueueError> {
		if !path.exists() {
			return Err(QueueError::NoTrack(path));
		}

		let tag = Tag::read_from_path(&path).unwrap_or_default();
		Ok(Track { path, tag })
	}

	pub fn maybe_deserialize<'de, D>(data: D) -> Result<Option<Track>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let path_or: Option<Utf8PathBuf> = Deserialize::deserialize(data)?;
		let track = path_or.and_then(|path| Track::try_from(path).ok());
		Ok(track)
	}

	pub fn directory<P: AsRef<Utf8Path>>(path: P) -> Result<Vec<Self>, QueueError> {
		let path = path.as_ref();
		if !path.is_dir() {
			return Err(QueueError::NotADirectory(path.to_owned()));
		}

		let files = fs::read_dir(path)?;
		let (dirs, files): (Vec<_>, Vec<_>) = files
			.into_iter()
			.flatten()
			.map(|entry| entry.path())
			.flat_map(Utf8PathBuf::try_from)
			.partition(|path| path.is_dir());

		let recurse_tracks = dirs.into_iter().map(Track::directory).flatten_ok();
		let tracks = files
			.into_iter()
			.filter(|path| path.extension().map_or(false, |ext| ext == "mp3"))
			.map(Track::new);

		recurse_tracks.chain(tracks).collect()
	}

	pub fn as_str(&self) -> &str {
		self.path.as_str()
	}

	pub fn track(&self) -> Option<u32> {
		self.tag.track()
	}

	pub fn line(&self, queue: &Queue) -> Line {
		let fmt = self.to_string();
		if let Some(track) = queue.track() {
			if track == self {
				Line::styled(fmt, utils::style::accent().bold())
			} else {
				Line::from(fmt)
			}
		} else {
			Line::from(fmt)
		}
	}

	pub fn title(&self) -> Option<String> {
		self.tag.title().map(ToOwned::to_owned)
	}

	pub fn artist(&self) -> Option<String> {
		self.tag.artist().map(ToOwned::to_owned)
	}

	pub fn album(&self) -> Option<String> {
		self.tag.album().map(ToOwned::to_owned)
	}

	pub fn lyrics(&self) -> Option<String> {
		self.tag.lyrics().next().map(ToString::to_string)
	}
}

impl Debug for Track {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut dbg = f.debug_struct("Track");
		dbg.field("path", &self.path);

		self.tag.track().map(|track| dbg.field("track", &track));
		self.tag.title().map(|title| dbg.field("title", &title));
		self.tag.artist().map(|artist| dbg.field("artist", &artist));
		self.tag.album().map(|album| dbg.field("album", &album));

		dbg.finish()
	}
}

// todo perhaps album
// todo "no title", "no artist"
impl Display for Track {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let track = self
			.tag
			.track()
			.map_or(String::new(), |track| format!("{:#02} ", track));
		let title = self.tag.title().unwrap_or("");
		let artist = self.tag.artist().unwrap_or("");

		write!(f, "{}{} ~ {}", track, title, artist)
	}
}

impl TryFrom<Utf8PathBuf> for Track {
	type Error = QueueError;
	fn try_from(path: Utf8PathBuf) -> Result<Self, Self::Error> {
		Track::new(path)
	}
}

impl TryFrom<&str> for Track {
	type Error = QueueError;
	fn try_from(string: &str) -> Result<Self, Self::Error> {
		let path = Utf8PathBuf::from(string);
		Track::try_from(path)
	}
}

impl Eq for Track {}

impl PartialEq<Track> for Track {
	fn eq(&self, other: &Track) -> bool {
		self.path.eq(&other.path)
	}
}

impl PartialEq<Utf8PathBuf> for Track {
	fn eq(&self, other: &Utf8PathBuf) -> bool {
		self.path.eq(other)
	}
}

impl PartialEq<&Utf8Path> for Track {
	fn eq(&self, other: &&Utf8Path) -> bool {
		self.path.eq(other)
	}
}

impl Ord for Track {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		let tracks = self.tag.track().zip(other.tag.track());
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

		tracks
			.map_or(Ordering::Equal, |(s, o)| s.cmp(&o))
			.then_with(|| titles.map_or(Ordering::Equal, |(s, o)| s.cmp(&o)))
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
	path: Option<Utf8PathBuf>,
	tracks: Vec<Track>,
	last: VecDeque<Track>,
	next: Vec<Track>,
	current: Option<Track>,
	shuffle: bool,
	rng: ThreadRng,
}

impl Queue {
	pub fn state(state: &State) -> color_eyre::Result<Self> {
		let (tracks, path) = match state.queue.as_deref() {
			Some(path) if path.exists() => {
				let mut tracks = Track::directory(path)?;
				tracks.sort();

				(tracks, Some(path.to_owned()))
			}
			_ => (vec![], None),
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

		let queue = Queue {
			path,
			tracks,
			last,
			next,
			current,
			shuffle,
			rng,
		};
		Ok(queue)
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
	pub fn path(&self) -> Option<&Utf8PathBuf> {
		self.path.as_ref()
	}

	#[inline]
	pub fn tracks(&self) -> &[Track] {
		&self.tracks
	}

	#[inline]
	pub fn track(&self) -> Option<&Track> {
		self.current.as_ref()
	}

	#[inline]
	pub fn idx(&self) -> Option<usize> {
		self.track()
			.and_then(|track| self.tracks().iter().position(|map| track == map))
	}

	#[inline]
	fn track_by_idx(&mut self, idx: usize) -> Result<Track, QueueError> {
		let track = self.tracks.get(idx).ok_or(QueueError::OutOfBounds(idx))?;
		Ok(track.clone())
	}

	pub fn queue<P: AsRef<Utf8Path> + Into<Utf8PathBuf>>(
		&mut self,
		path: P,
	) -> Result<(), QueueError> {
		let mut tracks = Track::directory(&path)?;
		tracks.sort();

		self.path = Some(path.into());
		self.tracks = tracks;
		self.current = None;
		self.last.clear();
		self.next.clear();

		Ok(())
	}

	pub fn select_path(&mut self, path: &Utf8Path, player: &mut Player) {
		let track = self.tracks.iter().find(|&iter| iter == &path).unwrap();
		let track = track.clone();

		self.next.clear();
		self.last.clear();

		self.replace(track, player);
	}

	pub fn select_idx(&mut self, idx: usize, player: &mut Player) -> Result<(), QueueError> {
		let track = self.track_by_idx(idx)?;
		self.replace(track, player);

		self.next.clear();
		self.last.clear();

		Ok(())
	}

	fn last_track_sequential(&mut self) -> Option<Track> {
		if self.tracks.is_empty() {
			return None;
		}

		let len = self.tracks().len();
		let idx = self.idx();
		let idx = idx.map(|idx| {
			if idx == 0 {
				len.saturating_sub(1)
			} else {
				idx.saturating_sub(1)
			}
		});

		idx.and_then(|idx| self.track_by_idx(idx).ok())
	}

	pub fn last(&mut self, player: &mut Player) {
		let last = if let Some(last) = self.last.pop_back() {
			Some(last)
		} else if !self.shuffle {
			self.last_track_sequential()
		} else {
			None
		};

		if let Some(track) = last {
			player.replace(track.as_str());

			if let Some(current) = self.current.replace(track) {
				self.next.push(current);
			}
		}
	}

	fn next_track_sequential(&mut self) -> Result<Track, QueueError> {
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

		self.track_by_idx(idx)
	}

	fn next_track_shuffle(&mut self) -> Result<Track, QueueError> {
		if let Some(current) = self.track().cloned() {
			let track = self
				.tracks
				.iter()
				.filter(|&track| track != &current)
				.choose(&mut self.rng)
				.cloned()
				.unwrap_or(current);
			Ok(track)
		} else {
			self.tracks
				.iter()
				.choose(&mut self.rng)
				.cloned()
				.ok_or(QueueError::NoTracks)
		}
	}

	fn next_track(&mut self) -> Result<Track, QueueError> {
		if let Some(track) = self.next.pop() {
			Ok(track)
		} else if self.shuffle {
			self.next_track_shuffle()
		} else {
			self.next_track_sequential()
		}
	}

	fn replace(&mut self, track: Track, player: &mut Player) {
		player.replace(track.as_str());
		player.pause(false);

		if self.track() != Some(&track) {
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
		let track = self.next_track()?;
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

	pub fn seek_i(&mut self, player: &mut Player, state: &State, amt: Duration) {
		if self.current.is_some() {
			if let Some((elapsed, duration)) = state.elapsed_duration() {
				let position = elapsed.saturating_add(amt);

				if position >= duration {
					let _ = self.next(player);
				} else {
					player.seek(position);
				}
			}
		}
	}

	pub fn done(&mut self, player: &mut Player, state: &State) -> color_eyre::Result<()> {
		if state.done() {
			self.next(player)?;
		}

		Ok(())
	}
}
