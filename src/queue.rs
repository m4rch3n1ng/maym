//! queue and track

use crate::{
	player::{Playable, Player},
	state::State,
	ui::utils as ui,
};
use arrayvec::ArrayVec;
use camino::{Utf8Path, Utf8PathBuf};
use id3::{Tag, TagLike};
use ratatui::text::Line;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
	fmt::{Debug, Display},
	sync::Arc,
	time::Duration,
};
use thiserror::Error;
use unicase::UniCase;
use walkdir::WalkDir;

/// queue error
#[derive(Debug, Error)]
pub enum QueueError {
	/// track doesn't exist
	#[error("couldn't find track {0:?}")]
	NoTrack(Utf8PathBuf),
	/// queue is empty
	#[error("queue is empty")]
	NoTracks,
	/// path is a directory
	#[error("is directory: {0:?}")]
	IsDirectory(Utf8PathBuf),
	/// index is out of bounds
	#[error("index out of bounds")]
	OutOfBounds,
	/// path is not a directory
	#[error("not a directory {0:?}")]
	NotADirectory(Utf8PathBuf),
	/// io error
	#[error("io error")]
	IoError(#[from] std::io::Error),
}

/// struct representing a mp3 file
#[derive(Clone)]
pub struct Track(Arc<TrackInner>);

pub struct TrackInner {
	/// path to file
	pub path: Utf8PathBuf,
	/// id3 tags
	tag: Tag,
}

impl Serialize for Track {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.path.as_path().serialize(serializer)
	}
}

impl Track {
	/// read [`Track`] from path
	///
	/// # Errors
	///
	/// returns [`QueueError`] if the path doesn't exist or is a directory
	pub fn new(path: Utf8PathBuf) -> Result<Self, QueueError> {
		if !path.exists() {
			return Err(QueueError::NoTrack(path));
		} else if path.is_dir() {
			return Err(QueueError::IsDirectory(path));
		}

		let tag = Tag::read_from_path(&path).unwrap_or_default();
		let track = TrackInner { path, tag };
		Ok(Track(Arc::new(track)))
	}

	/// deserialize into [`Option`] of [`Track`]
	///
	/// serializes into [`None`] if path doesn't exist
	pub fn maybe_deserialize<'de, D>(data: D) -> Result<Option<Track>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let path_or: Option<Utf8PathBuf> = Deserialize::deserialize(data)?;
		let track = path_or.and_then(|path| Track::new(path).ok());
		Ok(track)
	}

	/// recursively read [`Track`]s from the given directory and sort them
	///
	/// # Errors
	///
	/// returns [`QueueError`] if path is not a directory
	pub fn directory<P: AsRef<Utf8Path>>(path: P) -> Result<Vec<Self>, QueueError> {
		let path = path.as_ref();
		if !path.is_dir() {
			return Err(QueueError::NotADirectory(path.to_owned()));
		}

		std::fs::read_dir(path)?;
		let mut tracks = WalkDir::new(path)
			.into_iter()
			.filter_map(Result::ok)
			.filter(|entry| entry.file_type().is_file())
			.map(|entry| entry.into_path())
			.filter_map(|x| Utf8PathBuf::try_from(x).ok())
			.filter(|path| path.extension() == Some("mp3"))
			.map(|path| Track::new(path).expect("should exist and not be a directory"))
			.collect::<Vec<_>>();

		tracks.sort();
		Ok(tracks)
	}

	/// format track into a [`ratatui::text::Line`] struct
	///
	/// takes [`Queue`] to highlight currently playing track
	pub fn line(&self, queue: &Queue) -> Line<'_> {
		let fmt = self.to_string();
		if let Some(track) = queue.track() {
			if track == self {
				ui::widgets::line(fmt, ui::style::accent().bold())
			} else {
				Line::from(fmt)
			}
		} else {
			Line::from(fmt)
		}
	}

	/// path to the mp3 file
	pub fn path(&self) -> &Utf8Path {
		&self.0.path
	}

	/// [id3 track tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#trck)
	pub fn track(&self) -> Option<u32> {
		self.0.tag.track()
	}

	/// reference to [id3 title tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#tit2)
	pub fn title(&self) -> Option<&str> {
		self.0.tag.title()
	}

	/// reference to [id3 artist tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#tpe1)
	pub fn artist(&self) -> Option<&str> {
		self.0.tag.artist()
	}

	/// reference to [id3 album tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#talb)
	pub fn album(&self) -> Option<&str> {
		self.0.tag.album()
	}

	/// reference to [id3 lyrics tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#uslt)
	pub fn lyrics(&self) -> Option<&str> {
		self.0.tag.lyrics().next().map(|lyr| &*lyr.text)
	}
}

impl Debug for Track {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut dbg = f.debug_struct("Track");
		dbg.field("path", &self.0.path);

		self.track().map(|track| dbg.field("track", &track));
		self.title().map(|title| dbg.field("title", &title));
		self.artist().map(|artist| dbg.field("artist", &artist));
		self.album().map(|album| dbg.field("album", &album));

		dbg.finish()
	}
}

impl Display for Track {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(track) = self.track() {
			write!(f, "{track:#02} ")?;
		}

		let title = self.title().unwrap_or("unknown title");
		let artist = self.artist().unwrap_or("unknown artist");

		write!(f, "{title} ~ {artist}")
	}
}

impl Eq for Track {}

impl PartialEq<Track> for Track {
	fn eq(&self, other: &Track) -> bool {
		self.0.path.eq(&other.0.path)
	}
}

impl PartialEq<Track> for Utf8PathBuf {
	fn eq(&self, other: &Track) -> bool {
		self.eq(&other.0.path)
	}
}

impl PartialEq<Utf8PathBuf> for Track {
	fn eq(&self, other: &Utf8PathBuf) -> bool {
		self.0.path.eq(other)
	}
}

impl PartialEq<Utf8Path> for Track {
	fn eq(&self, other: &Utf8Path) -> bool {
		self.0.path.eq(other)
	}
}

impl Ord for Track {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		let tracks = self.track().zip(other.track());
		let titles = self
			.title()
			.zip(other.title())
			.map(|(s, o)| (UniCase::new(s), UniCase::new(o)));
		let artist = self
			.artist()
			.zip(other.artist())
			.map(|(s, o)| (UniCase::new(s), UniCase::new(o)));
		let albums = self
			.album()
			.zip(other.album())
			.map(|(s, o)| (UniCase::new(s), UniCase::new(o)));

		(tracks.map_or(std::cmp::Ordering::Equal, |(s, o)| s.cmp(&o)))
			.then_with(|| titles.map_or(std::cmp::Ordering::Equal, |(s, o)| s.cmp(&o)))
			.then_with(|| artist.map_or(std::cmp::Ordering::Equal, |(s, o)| s.cmp(&o)))
			.then_with(|| albums.map_or(std::cmp::Ordering::Equal, |(s, o)| s.cmp(&o)))
	}
}

impl PartialOrd for Track {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Debug)]
struct History {
	queue: ArrayVec<usize, 100>,
	index: usize,
}

impl History {
	fn new() -> Self {
		History {
			queue: ArrayVec::new(),
			index: 0,
		}
	}

	/// should only be called if [`Self::next`] is None, otherwise
	/// weird shit will happen and it will crash in debug mode
	fn push(&mut self, value: usize) {
		debug_assert_eq!(self.queue.len().saturating_sub(1), self.index);
		if self.queue.last() == Some(&value) {
			return;
		}

		if self.queue.is_full() {
			self.queue.as_mut_slice().rotate_left(1);
			self.queue.pop();
		}

		self.queue.push(value);
		self.index = self.queue.len() - 1;
	}

	fn clear(&mut self) {
		self.queue.clear();
		self.index = 0;
	}

	fn next(&mut self) -> Option<usize> {
		let next = self.queue.get(self.index + 1)?;
		self.index += 1;
		Some(*next)
	}

	fn prev(&mut self) -> Option<usize> {
		let prev = self.index.checked_sub(1)?;
		self.index = prev;
		Some(self.queue[prev])
	}
}

/// struct managing playback queue
#[derive(Debug)]
pub struct Queue {
	/// queue path
	path: Option<Utf8PathBuf>,
	/// queue track list
	tracks: Vec<Track>,
	/// queue history
	history: History,
	/// currently playing track
	current: Option<usize>,
	/// do shuffle queue
	shuffle: bool,
}

impl Queue {
	/// initialize [`Queue`] with a [`State`] struct
	pub fn with_state(state: &State) -> color_eyre::Result<Self> {
		let (tracks, path) = if let Some(path) = state.queue.as_deref()
			&& path.exists()
		{
			let tracks = Track::directory(path)?;
			(tracks, Some(path.to_owned()))
		} else {
			(Vec::new(), None)
		};

		let current = (state.track.as_ref())
			.and_then(|current| tracks.iter().position(|track| track == current));

		let mut history = History::new();
		if let Some(index) = current {
			history.push(index);
		}

		let queue = Queue {
			path,
			tracks,
			history,
			current,
			shuffle: state.shuffle,
		};
		Ok(queue)
	}

	/// returns if shuffle is active
	#[inline]
	pub fn is_shuffle(&self) -> bool {
		self.shuffle
	}

	/// toggle shuffle
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	pub fn shuffle(&mut self) {
		self.history.clear();
		self.shuffle = !self.shuffle;
	}

	/// set shuffle
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	#[cfg(feature = "mpris")]
	pub fn set_shuffle(&mut self, shuffle: bool) {
		if self.shuffle != shuffle {
			self.history.clear();
			self.shuffle = shuffle;
		}
	}

	/// return queue path
	#[inline]
	pub fn path(&self) -> Option<&Utf8Path> {
		self.path.as_deref()
	}

	/// return track list
	#[inline]
	pub fn tracks(&self) -> &[Track] {
		&self.tracks
	}

	/// return currently playing track
	#[inline]
	pub fn track(&self) -> Option<&Track> {
		self.current.map(|idx| &self.tracks[idx])
	}

	/// return index of currently playing track
	#[inline]
	pub fn index(&self) -> Option<usize> {
		self.current
	}

	/// queue a new directory
	///
	/// # Errors
	///
	/// returns [`QueueError`] if the directory doesn't exist
	pub fn queue<P: AsRef<Utf8Path> + Into<Utf8PathBuf>>(
		&mut self,
		path: P,
	) -> Result<(), QueueError> {
		let tracks = Track::directory(&path)?;

		self.path = Some(path.into());
		self.tracks = tracks;
		self.current = None;
		self.history.clear();

		Ok(())
	}

	/// select track by path
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	///
	/// # Errors
	///
	/// returns [`QueueError`] if the track of the path isn't in the [`Queue::tracks`]
	pub fn select_path<P: Playable>(
		&mut self,
		path: &Utf8Path,
		player: &mut P,
	) -> Result<(), QueueError> {
		let Some(index) = self.tracks.iter().position(|iter| iter == path) else {
			return Err(QueueError::NoTrack(path.to_owned()));
		};

		self.replace(index, player);

		self.history.clear();

		Ok(())
	}

	/// select track by index
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	///
	/// # Errors
	///
	/// returns [`QueueError`] if the index is out bounds
	pub fn select_idx<P: Playable>(
		&mut self,
		index: usize,
		player: &mut P,
	) -> Result<(), QueueError> {
		self.tracks.get(index).ok_or(QueueError::OutOfBounds)?;
		self.replace(index, player);

		self.history.clear();

		Ok(())
	}

	/// select last track sequentially
	///
	/// returns [`None`] on an empty track list,
	/// or if no track is currently playing
	fn last_track_sequential(&self) -> Option<usize> {
		if self.tracks.is_empty() {
			return None;
		}

		self.current.map(|idx| {
			if idx == 0 {
				self.tracks.len().saturating_sub(1)
			} else {
				idx.saturating_sub(1)
			}
		})
	}

	/// play last track
	///
	/// in order:
	/// 1. try to pop from [`Queue::last`]
	/// 2. if !self.shuffle, use [`Queue::last_track_sequential`]
	/// 3. give up
	///
	/// if it finds a track to play, it pushes it to [`Queue::next`]
	pub fn last<P: Playable>(&mut self, player: &mut P) {
		let last = if let Some(last) = self.history.prev() {
			Some(last)
		} else if !self.shuffle {
			self.last_track_sequential()
		} else {
			None
		};

		if let Some(index) = last {
			self.replace(index, player);
		}
	}

	/// get next track sequentially
	///
	/// # Errors
	///
	/// returns [`QueueError`] if [`Queue::tracks`] is empty
	fn next_track_sequential(&self) -> Option<usize> {
		if self.tracks.is_empty() {
			return None;
		}

		let idx = self.current.map_or(0, |idx| (idx + 1) % self.tracks.len());
		Some(idx)
	}

	/// get next track randomly
	///
	/// # Errors
	///
	/// returns [`QueueError`] if [`Queue::tracks`] is empty
	fn next_track_shuffle(&self) -> Option<usize> {
		if self.tracks.is_empty() {
			return None;
		} else if self.tracks.len() <= 1 {
			return Some(0);
		}

		loop {
			let track = rand::random_range(..self.tracks.len());
			if self.current.is_none_or(|current| current != track) {
				return Some(track);
			}
		}
	}

	/// get next track
	fn next_track(&mut self) -> Option<usize> {
		if let Some(track) = self.history.next() {
			Some(track)
		} else if !self.shuffle {
			self.next_track_sequential()
		} else if let Some(index) = self.next_track_shuffle() {
			self.history.push(index);
			Some(index)
		} else {
			None
		}
	}

	/// replace current track
	///
	/// replaces track in [`Player`] via [`Player::replace`]
	/// and pushes last track to [`Queue::last`]
	fn replace<P: Playable>(&mut self, index: usize, player: &mut P) {
		player.replace(&self.tracks[index]);
		self.current = Some(index);
	}

	/// play next track
	pub fn next<P: Playable>(&mut self, player: &mut P) {
		if let Some(track) = self.next_track() {
			self.replace(track, player);
		}
	}

	/// restart current track
	pub fn restart(&self, player: &mut Player) {
		if self.current.is_some() {
			let start = Duration::ZERO;
			player.seek(start);
		}
	}

	/// seek backwards in current track
	pub fn seek_d(&self, player: &mut Player, state: &State, amt: Duration) {
		if self.current.is_some()
			&& let Some(elapsed) = state.elapsed()
		{
			let position = elapsed.saturating_sub(amt);
			player.seek(position);
		}
	}

	/// seek forward in current track
	pub fn seek_i(&mut self, player: &mut Player, state: &State, amt: Duration) {
		if self.current.is_some()
			&& let Some((elapsed, duration)) = state.elapsed_duration()
		{
			let position = elapsed.saturating_add(amt);

			if position >= duration {
				self.next(player);
			} else {
				player.seek(position);
			}
		}
	}

	/// if [`State::done()`], play next track
	pub fn done(&mut self, player: &mut Player) {
		if player.done() {
			self.next(player);
		}
	}
}

#[cfg(test)]
mod test {
	use super::{History, Queue, QueueError, Track};
	use crate::{player::Playable, state};
	use camino::{Utf8Path, Utf8PathBuf};
	use std::cmp::Ordering;

	struct Player;

	impl Player {
		fn new() -> Player {
			Player
		}
	}

	impl Playable for Player {
		fn replace(&mut self, _track: &Track) {}
	}

	/// create [`Track`] by reading from disk
	///
	/// # Errors
	///
	/// returns error when path doesn't exist or is directory
	fn track<P: Into<Utf8PathBuf>>(path: P) -> Result<Track, QueueError> {
		let path = path.into();
		Track::new(path)
	}

	/// read path into vec of [`Track`]
	///
	/// # Errors
	///
	/// returns error when path doesn't exist or is not a directory
	fn list<P: AsRef<Utf8Path>>(path: P) -> Result<Vec<Track>, QueueError> {
		let tracks = Track::directory(path)?;
		Ok(tracks)
	}

	/// create mock [`Queue`] in path
	///
	/// # Errors
	///
	/// returns error when path doesn't exist or is not a directory
	fn queue<P: Into<Utf8PathBuf>>(path: P) -> Result<Queue, QueueError> {
		let path = path.into();

		let tracks = Track::directory(&path)?;
		let queue = Queue {
			path: Some(path),
			tracks,
			history: History::new(),
			current: None,
			shuffle: false,
		};
		Ok(queue)
	}

	#[test]
	fn seq() -> color_eyre::Result<()> {
		let t0 = track("mock/list 01/track 00.mp3")?;
		let t1 = track("mock/list 01/track 01.mp3")?;
		let t2 = track("mock/list 01/sub 02/track 02.mp3")?;
		let t5 = track("mock/list 01/sub 01/track 05.mp3")?;

		let mut player = Player::new();
		let mut queue = queue("mock/list 01")?;

		queue.next(&mut player);
		assert_eq!(queue.track(), Some(&t0));

		queue.next(&mut player);
		assert_eq!(queue.track(), Some(&t1));

		queue.next(&mut player);
		assert_eq!(queue.track(), Some(&t2));

		queue.next(&mut player);
		queue.next(&mut player);
		queue.next(&mut player);
		queue.next(&mut player);

		assert_eq!(queue.track(), Some(&t0));

		queue.last(&mut player);
		assert_eq!(queue.track(), Some(&t5));

		Ok(())
	}

	#[test]
	fn last_seq() -> color_eyre::Result<()> {
		let t1 = track("mock/list 01/track 01.mp3")?;
		let t2 = track("mock/list 01/sub 02/track 02.mp3")?;
		let t5 = track("mock/list 01/sub 01/track 05.mp3")?;

		let mut player = Player::new();
		let mut queue = queue("mock/list 01")?;

		assert_eq!(queue.track(), None);

		queue.next(&mut player);
		assert_eq!(queue.history.queue.len(), 0);

		queue.last(&mut player);
		assert_eq!(queue.track(), Some(&t5));

		queue.next(&mut player);
		queue.next(&mut player);
		queue.next(&mut player);

		assert_eq!(queue.track(), Some(&t2));

		queue.last(&mut player);
		assert_eq!(queue.track(), Some(&t1));

		Ok(())
	}

	#[test]
	fn shuf() -> color_eyre::Result<()> {
		let mut player = Player::new();
		let mut queue = queue("mock/list 01")?;

		queue.shuffle();
		assert!(queue.is_shuffle());

		queue.next(&mut player);
		queue.next(&mut player);
		queue.next(&mut player);

		let tt = queue.current;

		queue.next(&mut player);
		queue.last(&mut player);
		queue.last(&mut player);
		queue.next(&mut player);

		assert_eq!(queue.current, tt);
		assert_eq!(queue.history.index, 2);
		assert_eq!(queue.history.queue.len(), 4);

		queue.shuffle();
		assert!(!queue.is_shuffle());
		assert!(queue.history.queue.is_empty());

		Ok(())
	}

	#[test]
	fn idx() -> color_eyre::Result<()> {
		let t1 = track("mock/list 01/track 01.mp3")?;
		let t2 = track("mock/list 01/sub 02/track 02.mp3")?;

		let mut player = Player::new();
		let mut queue = queue("mock/list 01")?;

		queue.next(&mut player);
		queue.next(&mut player);
		queue.next(&mut player);
		queue.last(&mut player);

		queue.select_idx(2, &mut player)?;
		assert_eq!(queue.track(), Some(&t2));

		assert!(queue.history.queue.is_empty());

		queue.select_idx(1, &mut player)?;
		assert_eq!(queue.track(), Some(&t1));

		Ok(())
	}

	#[test]
	fn path() -> color_eyre::Result<()> {
		let t0 = track("mock/list 01/track 00.mp3")?;
		let t4 = track("mock/list 01/sub 01/track 04.mp3")?;

		let mut player = Player::new();
		let mut queue = queue("mock/list 01")?;

		queue.next(&mut player);
		queue.next(&mut player);
		queue.next(&mut player);
		queue.last(&mut player);

		queue.select_path("mock/list 01/sub 01/track 04.mp3".into(), &mut player)?;
		assert_eq!(queue.track(), Some(&t4));

		assert!(queue.history.queue.is_empty());

		queue.select_path("mock/list 01/track 00.mp3".into(), &mut player)?;
		assert_eq!(queue.track(), Some(&t0));

		Ok(())
	}

	#[test]
	fn dot_queue() -> color_eyre::Result<()> {
		let mut queue = queue("mock/list 01")?;
		let list02 = list("mock/list 02")?;

		assert_eq!(queue.tracks().len(), 6);

		queue.queue("mock/list 02")?;
		assert_eq!(queue.tracks, list02);
		assert_eq!(queue.tracks().len(), 5);

		Ok(())
	}

	#[test]
	fn queue_state() -> color_eyre::Result<()> {
		let empty = state::test::mock::<&str>(None, None)?;
		let queue = Queue::with_state(&empty)?;

		assert!(queue.path.is_none());
		assert!(queue.tracks.is_empty());
		assert!(queue.current.is_none());

		let no_exists = state::test::mock(Some("mock/list 04"), Some("mock/list 01/track 01.mp3"))?;
		let queue = Queue::with_state(&no_exists)?;

		assert!(queue.path.is_none());
		assert!(queue.tracks.is_empty());
		assert!(queue.current.is_none());

		let no_track = state::test::mock(Some("mock/list 01"), None)?;
		let queue = Queue::with_state(&no_track)?;

		assert_eq!(queue.path, Some("mock/list 01".into()));
		assert_eq!(queue.tracks.len(), 6);
		assert!(queue.current.is_none());

		let track_not_in_list =
			state::test::mock(Some("mock/list 01"), Some("mock/list 02/track 01.mp3"))?;
		let queue = Queue::with_state(&track_not_in_list)?;

		assert!(queue.path.is_some());
		assert_eq!(queue.tracks.len(), 6);
		assert!(queue.current.is_none());

		let exists = state::test::mock(Some("mock/list 01"), Some("mock/list 01/track 01.mp3"))?;
		let track = Track::new("mock/list 01/track 01.mp3".into())?;
		let queue = Queue::with_state(&exists)?;

		assert!(queue.path.is_some());
		assert_eq!(queue.tracks.len(), 6);
		assert_eq!(queue.track(), Some(&track));

		Ok(())
	}

	/// create [`serde_json`] string deserializer
	fn deserializer(val: &str) -> serde_json::de::Deserializer<serde_json::de::StrRead<'_>> {
		serde_json::de::Deserializer::from_str(val)
	}

	#[test]
	fn maybe_deserialize() -> Result<(), serde_json::Error> {
		let mut des = deserializer("null");
		let track = Track::maybe_deserialize(&mut des)?;
		assert_eq!(track, None);

		let mut des = deserializer("\"mock/list 01/track 08.mp3\"");
		let track = Track::maybe_deserialize(&mut des)?;
		assert!(track.is_none());

		Ok(())
	}

	/// create mock [`Track`] by setting the tags directly
	///
	/// # Usage
	///
	/// ```
	/// // set track
	/// track!(#1);
	/// // set track and title
	/// track!(#1, "title");
	/// // set track, title and artist
	/// track!(#1, "title", "artist");
	/// // set track, title, artist and album
	/// track!(#1, "title", "artist", "album");
	/// // set title
	/// track!("title");
	/// // set title and artist
	/// track!("title", "artist");
	/// // set title, artist and album
	/// track!("title", "artist", "album");
	/// // set artist
	/// track!(#1, art = "artist");
	/// // set artist and album
	/// track!(#1, art = "artist", alb = "album");
	/// // set title and album
	/// track!(#1, tit = "title", alb = "album");
	/// // set album
	/// track!(#1, alb = "album");
	/// ```
	macro_rules! track {
		($(# $tr:expr, )? $(tit = $tit:expr, )? $(art = $art:expr, )? $(alb = $alb:expr, )?) => {
			{
				use id3::{Tag, TagLike};

				let mut tag = Tag::new();
				$( tag.set_track($tr); )?
				$( tag.set_title($tit); )?
				$( tag.set_artist($art); )?
				$( tag.set_album($alb); )?

				let path = "/dev/null".into();
				let track = super::TrackInner { path, tag };
				let track = Track(std::sync::Arc::new(track));

				track
			}
		};
		(# $tr:expr $(, $tit:expr $(, $art:expr $(, $alb:expr)?)?)?) => {
			track!(
				# $tr,
				$( tit = $tit,
				$( art = $art,
				$( alb = $alb, )? )? )?
			)
		};
		($tit:expr $(, $art:expr $(, $alb:expr)?)?) => {
			track!(
				tit = $tit,
				$( art = $art,
				$( alb = $alb, )? )?
			)
		};
	}

	#[test]
	fn ord() {
		let one = track!(#0);
		let two = track!("00", "00");
		let thr = track!(#1, "01");
		let fou = track!(art = "01",);
		let fiv = track!(#0, "00", "02");

		assert_eq!(one.cmp(&two), Ordering::Equal);
		assert_eq!(one.cmp(&thr), Ordering::Less);
		assert_eq!(two.cmp(&thr), Ordering::Less);
		assert_eq!(one.cmp(&fou), Ordering::Equal);
		assert_eq!(two.cmp(&fou), Ordering::Less);
		assert_eq!(fou.cmp(&fiv), Ordering::Less);
		assert_eq!(thr.cmp(&fiv), Ordering::Greater);
		assert_eq!(one.cmp(&fiv), Ordering::Equal);
		assert_eq!(two.cmp(&fou), Ordering::Less);
	}

	#[test]
	fn ord_case() {
		let one = track!(#0, "a");
		let two = track!("B", "c");
		let thr = track!(#1, art = "D",);
		let fou = track!("c");

		assert_eq!(one.cmp(&two), Ordering::Less);
		assert_eq!(two.cmp(&one), Ordering::Greater);
		assert_eq!(two.cmp(&thr), Ordering::Less);
		assert_eq!(thr.cmp(&two), Ordering::Greater);
		assert_eq!(two.cmp(&fou), Ordering::Less);
		assert_eq!(fou.cmp(&two), Ordering::Greater);
	}

	#[test]
	fn ord_unicode() {
		let one = track!("ä");
		let two = track!("Ü", "ẞ");
		let thr = track!("Ä");
		let fou = track!("ü", "ss");

		assert_eq!(one.cmp(&two), Ordering::Less);
		assert_eq!(two.cmp(&one), Ordering::Greater);
		assert_eq!(thr.cmp(&fou), Ordering::Less);
		assert_eq!(fou.cmp(&thr), Ordering::Greater);

		assert_eq!(one.cmp(&thr), Ordering::Equal);
		assert_eq!(thr.cmp(&one), Ordering::Equal);
		assert_eq!(two.cmp(&fou), Ordering::Equal);
		assert_eq!(fou.cmp(&two), Ordering::Equal);
	}
}
