//! queue and track

use crate::{player::Player, state::State, ui::utils as ui};
use camino::{Utf8Path, Utf8PathBuf};
use id3::{Tag, TagLike};
use itertools::Itertools;
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use ratatui::{style::Stylize, text::Line};
use serde::{Deserialize, Deserializer, Serialize};
use std::{cmp::Ordering, collections::VecDeque, fmt::Debug, fmt::Display, fs, time::Duration};
use thiserror::Error;
use unicase::UniCase;

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
	#[error("out of bounds {0}")]
	OutOfBounds(usize),
	/// path is not a directory
	#[error("not a directory {0:?}")]
	NotADirectory(Utf8PathBuf),
	/// io error
	#[error("io error")]
	IoError(#[from] std::io::Error),
}

/// struct representing a mp3 file
#[derive(Clone)]
pub struct Track {
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
		self.path.as_path().serialize(serializer)
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
		Ok(Track { path, tag })
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

	/// recursively read [`Track`]s from directory
	///
	/// # Errors
	///
	/// returns [`QueueError`] if path is not a directory
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
			.filter(|path| path.extension() == Some("mp3"))
			.map(Track::new);

		recurse_tracks.chain(tracks).collect()
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

	/// [id3 track tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#trck)
	pub fn track(&self) -> Option<u32> {
		self.tag.track()
	}

	/// reference to [id3 title tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#tit2)
	pub fn title(&self) -> Option<&str> {
		self.tag.title()
	}

	/// reference to [id3 artist tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#tpe1)
	pub fn artist(&self) -> Option<&str> {
		self.tag.artist()
	}

	/// reference to [id3 album tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#talb)
	pub fn album(&self) -> Option<&str> {
		self.tag.album()
	}

	/// reference to [id3 lyrics tag](https://mutagen-specs.readthedocs.io/en/latest/id3/id3v2.4.0-frames.html#uslt)
	pub fn lyrics(&self) -> Option<&str> {
		self.tag.lyrics().next().map(|lyr| &*lyr.text)
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

impl Display for Track {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(track) = self.tag.track() {
			write!(f, "{track:#02} ")?;
		}

		let title = self.tag.title().unwrap_or("unknown title");
		let artist = self.tag.artist().unwrap_or("unknown artist");

		write!(f, "{title} ~ {artist}")
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

impl PartialEq<Utf8Path> for Track {
	fn eq(&self, other: &Utf8Path) -> bool {
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
			.map(|(s, o)| (UniCase::new(s), UniCase::new(o)));
		let artist = self
			.tag
			.artist()
			.zip(other.tag.artist())
			.map(|(s, o)| (UniCase::new(s), UniCase::new(o)));
		let albums = self
			.tag
			.album()
			.zip(other.tag.album())
			.map(|(s, o)| (UniCase::new(s), UniCase::new(o)));

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

/// struct managing playback queue
#[derive(Debug)]
pub struct Queue {
	/// queue path
	path: Option<Utf8PathBuf>,
	/// queue track list
	tracks: Vec<Track>,
	/// previously played tracks
	last: VecDeque<Track>,
	/// next-up tracks
	next: Vec<Track>,
	/// currently playing track
	current: Option<Track>,
	/// do shuffle queue
	shuffle: bool,
	/// rng struct for shuffling
	rng: ThreadRng,
}

impl Queue {
	/// initialize [`Queue`] with a [`State`] struct
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

	/// returns if shuffle is active
	#[inline]
	pub fn is_shuffle(&self) -> bool {
		self.shuffle
	}

	/// toggle shuffle
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	pub fn shuffle(&mut self) {
		self.next.clear();
		self.last.clear();

		self.shuffle = !self.shuffle;
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
		self.current.as_ref()
	}

	/// return index of currently playing track, if it exists
	#[inline]
	pub fn idx(&self) -> Option<usize> {
		self.track()
			.and_then(|track| self.tracks().iter().position(|map| track == map))
	}

	/// internal implementation for [`Queue::select_idx`]
	#[inline]
	fn track_by_idx(&mut self, idx: usize) -> Result<Track, QueueError> {
		let track = self.tracks.get(idx).ok_or(QueueError::OutOfBounds(idx))?;
		Ok(track.clone())
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
		let mut tracks = Track::directory(&path)?;
		tracks.sort();

		self.path = Some(path.into());
		self.tracks = tracks;
		self.current = None;
		self.last.clear();
		self.next.clear();

		Ok(())
	}

	/// select track by path
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	///
	/// # Errors
	///
	/// returns [`QueueError`] if the track of the path isn't in the [`Queue::tracks`]
	pub fn select_path(&mut self, path: &Utf8Path, player: &mut Player) -> Result<(), QueueError> {
		let Some(track) = self.tracks.iter().find(|&iter| iter == path).cloned() else {
			return Err(QueueError::NoTrack(path.to_owned()));
		};

		self.replace(track, player);

		self.next.clear();
		self.last.clear();

		Ok(())
	}

	/// select track by index
	///
	/// also clears [`Queue::next`] and [`Queue::last`]
	///
	/// # Errors
	///
	/// returns [`QueueError`] if the index is out bounds
	pub fn select_idx(&mut self, idx: usize, player: &mut Player) -> Result<(), QueueError> {
		let track = self.track_by_idx(idx)?;
		self.replace(track, player);

		self.next.clear();
		self.last.clear();

		Ok(())
	}

	/// select last track sequentially
	///
	/// returns [`None`] on an empty track list,
	/// or if no track is currently playing
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

	/// play last track
	///
	/// in order:
	/// 1. try to pop from [`Queue::last`]
	/// 2. if !self.shuffle, use [`Queue::last_track_sequential`]
	/// 3. give up
	///
	/// if it finds a track to play, it pushes it to [`Queue::next`]
	pub fn last(&mut self, player: &mut Player) {
		let last = if let Some(last) = self.last.pop_back() {
			Some(last)
		} else if !self.shuffle {
			self.last_track_sequential()
		} else {
			None
		};

		if let Some(track) = last {
			player.replace(&track);

			if let Some(current) = self.current.replace(track) {
				self.next.push(current);
			}
		}
	}

	/// get next track sequentially
	///
	/// # Errors
	///
	/// returns [`QueueError`] if [`Queue::tracks`] is empty
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

	/// get next track randomly
	///
	/// # Errors
	///
	/// returns [`QueueError`] if [`Queue::tracks`] is empty
	fn next_track_shuffle(&mut self) -> Result<Track, QueueError> {
		if let Some(current) = self.current.as_ref() {
			// try to choose a different track if one is already playing
			// fall back when the playlist length is 1
			let track = self
				.tracks
				.iter()
				.filter(|&track| track != current)
				.choose(&mut self.rng)
				.cloned()
				.unwrap_or_else(|| current.clone());
			Ok(track)
		} else {
			self.tracks
				.iter()
				.choose(&mut self.rng)
				.cloned()
				.ok_or(QueueError::NoTracks)
		}
	}

	/// get next track
	fn next_track(&mut self) -> Result<Track, QueueError> {
		if let Some(track) = self.next.pop() {
			Ok(track)
		} else if self.shuffle {
			self.next_track_shuffle()
		} else {
			self.next_track_sequential()
		}
	}

	/// replace current track
	///
	/// replaces track in [`Player`] via [`Player::replace`]
	/// and pushes last track to [`Queue::last`]
	fn replace(&mut self, track: Track, player: &mut Player) {
		player.replace(&track);

		// only replace and add to last, if it isn't already playing
		// (i.e. it hasn't yet been added to last)
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

	/// play next track
	pub fn next(&mut self, player: &mut Player) -> Result<(), QueueError> {
		let track = self.next_track()?;
		self.replace(track, player);

		Ok(())
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
		if self.current.is_some() {
			if let Some(elapsed) = state.elapsed() {
				let position = elapsed.saturating_sub(amt);
				player.seek(position);
			}
		}
	}

	/// seek forward in current track
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

	/// if [`State::done()`], play next track
	pub fn done(&mut self, player: &mut Player) -> Result<(), QueueError> {
		if player.done() {
			self.next(player)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::{Queue, QueueError, Track};
	use crate::{player::Player, state};
	use camino::{Utf8Path, Utf8PathBuf};
	use std::{cmp::Ordering, collections::VecDeque};

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
		let mut list = Track::directory(path)?;
		list.sort();
		Ok(list)
	}

	/// create mock [`Queue`] in path
	///
	/// # Errors
	///
	/// returns error when path doesn't exist or is not a directory
	fn queue<P: Into<Utf8PathBuf>>(path: P) -> Result<Queue, QueueError> {
		let path = path.into();

		let mut tracks = Track::directory(&path)?;
		tracks.sort();

		let queue = Queue {
			path: Some(path),
			tracks,
			last: VecDeque::new(),
			next: vec![],
			current: None,
			shuffle: false,
			rng: rand::thread_rng(),
		};
		Ok(queue)
	}

	#[test]
	fn seq() -> color_eyre::Result<()> {
		let t0 = track("mock/list 01/track 00.mp3")?;
		let t1 = track("mock/list 01/track 01.mp3")?;
		let t2 = track("mock/list 01/sub 02/track 02.mp3")?;
		let t5 = track("mock/list 01/sub 01/track 05.mp3")?;

		let mut player = Player::new()?;
		let mut queue = queue("mock/list 01")?;

		queue.next(&mut player)?;
		assert_eq!(queue.track(), Some(&t0));

		queue.next(&mut player)?;
		assert_eq!(queue.track(), Some(&t1));

		queue.next(&mut player)?;
		assert_eq!(queue.track(), Some(&t2));

		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.next(&mut player)?;

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

		let mut player = Player::new()?;
		let mut queue = queue("mock/list 01")?;

		queue.last(&mut player);
		assert_eq!(queue.track(), None);

		queue.next(&mut player)?;
		assert!(queue.last.is_empty());

		queue.last(&mut player);
		assert_eq!(queue.track(), Some(&t5));

		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.next(&mut player)?;

		assert_eq!(queue.track(), Some(&t2));

		queue.last(&mut player);
		assert_eq!(queue.track(), Some(&t1));

		Ok(())
	}

	#[test]
	fn shuf() -> color_eyre::Result<()> {
		let mut player = Player::new()?;
		let mut queue = queue("mock/list 01")?;

		queue.shuffle();
		assert!(queue.is_shuffle());

		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.next(&mut player)?;

		let tt = queue.current.clone();

		queue.next(&mut player)?;
		queue.last(&mut player);
		queue.last(&mut player);
		queue.next(&mut player)?;

		assert_eq!(queue.current, tt);
		assert_eq!(queue.next.len(), 1);
		assert_eq!(queue.last.len(), 2);

		queue.shuffle();
		assert!(!queue.is_shuffle());
		assert!(queue.last.is_empty());
		assert!(queue.next.is_empty());

		Ok(())
	}

	#[test]
	fn idx() -> color_eyre::Result<()> {
		let t1 = track("mock/list 01/track 01.mp3")?;
		let t2 = track("mock/list 01/sub 02/track 02.mp3")?;

		let mut player = Player::new()?;
		let mut queue = queue("mock/list 01")?;

		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.last(&mut player);

		queue.select_idx(2, &mut player)?;
		assert_eq!(queue.track(), Some(&t2));

		assert!(queue.next.is_empty());
		assert!(queue.last.is_empty());

		queue.select_idx(1, &mut player)?;
		assert_eq!(queue.track(), Some(&t1));

		Ok(())
	}

	#[test]
	fn path() -> color_eyre::Result<()> {
		let t0 = track("mock/list 01/track 00.mp3")?;
		let t4 = track("mock/list 01/sub 01/track 04.mp3")?;

		let mut player = Player::new()?;
		let mut queue = queue("mock/list 01")?;

		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.next(&mut player)?;
		queue.last(&mut player);

		queue.select_path("mock/list 01/sub 01/track 04.mp3".into(), &mut player)?;
		assert_eq!(queue.track(), Some(&t4));

		assert!(queue.next.is_empty());
		assert!(queue.last.is_empty());

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
		let queue = Queue::state(&empty)?;

		assert!(queue.path.is_none());
		assert!(queue.tracks.is_empty());
		assert!(queue.current.is_none());

		let no_exists = state::test::mock(Some("mock/list 04"), Some("mock/list 01/track 01.mp3"))?;
		let queue = Queue::state(&no_exists)?;

		assert!(queue.path.is_none());
		assert!(queue.tracks.is_empty());
		assert!(queue.current.is_none());

		let no_track = state::test::mock(Some("mock/list 01"), None)?;
		let queue = Queue::state(&no_track)?;

		assert_eq!(queue.path, Some("mock/list 01".into()));
		assert_eq!(queue.tracks.len(), 6);
		assert!(queue.current.is_none());

		let track_not_in_list =
			state::test::mock(Some("mock/list 01"), Some("mock/list 02/track 01.mp3"))?;
		let queue = Queue::state(&track_not_in_list)?;

		assert!(queue.path.is_some());
		assert_eq!(queue.tracks.len(), 6);
		assert!(queue.current.is_none());

		let exists = state::test::mock(Some("mock/list 01"), Some("mock/list 01/track 01.mp3"))?;
		let track = Track::new("mock/list 01/track 01.mp3".into())?;
		let queue = Queue::state(&exists)?;

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
				let track = Track { path, tag };

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
