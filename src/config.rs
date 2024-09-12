//! player config
//!
//! contains the [`Config`] struct
//! and all [`List`] management

use crate::{
	queue::{Queue, Track},
	ui::utils as ui,
};
use camino::{Utf8Path, Utf8PathBuf};
use ratatui::{
	style::{Color, Style, Stylize},
	text::Line,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
	borrow::Cow,
	fmt::Display,
	fs,
	ops::{Deref, DerefMut},
	path::PathBuf,
	str::FromStr,
	sync::LazyLock,
	time::Duration,
};
use thiserror::Error;
use unicase::UniCase;

/// path for config file
static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| CONFIG_DIR.join("config.json"));
/// path to config directory
pub static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(config_dir);

/// path to config directory
///
/// creates the directory if it doesn't exist
fn config_dir() -> PathBuf {
	let mut config = dirs::config_dir().expect("config directory should exist");
	config.push("maym");

	if config.exists() && !config.is_dir() {
		fs::remove_file(&config).unwrap();
		fs::create_dir_all(&config).unwrap();
	} else if !config.exists() {
		fs::create_dir_all(&config).unwrap();
	}

	config
}

/// config error
#[derive(Debug, Error)]
pub enum ConfigError {
	#[error("file {0:?} not found")]
	FileNotFound(PathBuf),
	/// io error
	#[error("io error")]
	IoError(#[source] std::io::Error),
	/// serde error
	#[error("serde error")]
	SerdeJsonError(#[from] serde_json::Error),
	/// list doesn't exist
	#[error("list {0:?} doesn't exist")]
	ListDoesntExist(Utf8PathBuf),
}

impl From<std::io::Error> for ConfigError {
	fn from(io: std::io::Error) -> Self {
		if let std::io::ErrorKind::NotFound = io.kind() {
			ConfigError::FileNotFound(CONFIG_PATH.clone())
		} else {
			ConfigError::IoError(io)
		}
	}
}

/// [`Child`] of [`List`]
///
/// created via [`List::children`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Child {
	/// list directory
	List(List),
	/// audio file
	Mp3(Utf8PathBuf),
}

impl Child {
	/// return name of child
	///
	/// the name is just the `file_name`
	/// and a trailing slash for directories
	fn name(&self) -> Cow<'_, str> {
		match *self {
			Child::List(ref list) => {
				let path = list.path.file_name().unwrap_or_else(|| list.path.as_str());
				let path = format!("{}/", path);
				Cow::Owned(path)
			}
			Child::Mp3(ref path) => {
				let path = path.file_name().unwrap_or_else(|| path.as_str());
				Cow::Borrowed(path)
			}
		}
	}

	/// returns list if child is [`Child::List`].
	pub fn list(&self) -> Option<&List> {
		match self {
			Child::List(list) => Some(list),
			Child::Mp3(_) => None,
		}
	}

	/// formats [`Child`] into a [`ratatui::text::Line`].
	///
	/// - lists are underlined
	/// - currently playing track / list is accented and bold
	/// - containing lists are only accented
	pub fn line(&self, queue: &Queue) -> Line {
		let name = self.name();
		match *self {
			Child::List(ref list) => {
				let underline = Style::default().underlined();
				let accent = ui::style::accent().underlined();
				if let Some(path) = queue.path() {
					if list == &path {
						ui::widgets::line(name, accent.bold())
					} else if list.contains(path) {
						ui::widgets::line(name, accent)
					} else {
						ui::widgets::line(name, underline)
					}
				} else {
					ui::widgets::line(name, underline)
				}
			}
			Child::Mp3(ref path) => {
				if let Some(track) = queue.track() {
					if track == path {
						ui::widgets::line(name, ui::style::accent().bold())
					} else {
						Line::raw(name)
					}
				} else {
					Line::raw(name)
				}
			}
		}
	}
}

impl PartialEq<List> for Child {
	fn eq(&self, other: &List) -> bool {
		match *self {
			Child::List(ref list) => list.eq(other),
			Child::Mp3(_) => false,
		}
	}
}

impl PartialEq<Track> for Child {
	fn eq(&self, other: &Track) -> bool {
		match *self {
			Child::List(_) => false,
			Child::Mp3(ref path) => path.eq(&other.path),
		}
	}
}

impl Ord for Child {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match (self, other) {
			(Child::List(l1), Child::List(l2)) => {
				UniCase::new(&l1.path).cmp(&UniCase::new(&l2.path))
			}
			(Child::Mp3(p1), Child::Mp3(p2)) => UniCase::new(&p1).cmp(&UniCase::new(&p2)),
			(Child::List(_), Child::Mp3(_)) => std::cmp::Ordering::Less,
			(Child::Mp3(_), Child::List(_)) => std::cmp::Ordering::Greater,
		}
	}
}

impl PartialOrd for Child {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

/// struct that represents a directory
#[derive(Debug, Clone)]
pub struct List {
	/// list path
	pub path: Utf8PathBuf,
	/// parent list
	parent: Option<Box<List>>,
}

impl List {
	/// create [`List`] without parent.
	fn new(path: Utf8PathBuf) -> Result<Self, ConfigError> {
		if path.exists() {
			let list = List { path, parent: None };
			Ok(list)
		} else {
			Err(ConfigError::ListDoesntExist(path))
		}
	}

	/// create [`List`] struct with parent node
	pub fn with_parent(path: Utf8PathBuf, parent: List) -> Result<Self, ConfigError> {
		if path.exists() {
			let parent = Box::new(parent);
			let list = List {
				path,
				parent: Some(parent),
			};
			Ok(list)
		} else {
			Err(ConfigError::ListDoesntExist(path))
		}
	}

	/// extract parent from [`List`], if list has parent
	pub fn parent(&mut self) -> Option<List> {
		// i can take the parent, as this list should be discarded
		// if you want to get an owned version of the parent
		self.parent.take().map(|bx| *bx)
	}

	// todo error handling
	/// reads files in [`List`] and returns a vec of [`Child`]
	pub fn children(&self) -> Vec<Child> {
		let read = fs::read_dir(&self.path).unwrap();
		let mut children = read
			.flatten()
			// todo display non utf8
			.map(|entry| entry.path())
			.flat_map(Utf8PathBuf::try_from)
			.filter_map(|path| {
				if path.is_dir() {
					let list = List::with_parent(path, self.clone()).unwrap();
					let child = Child::List(list);
					Some(child)
				} else if path.extension() == Some("mp3") {
					let child = Child::Mp3(path);
					Some(child)
				} else {
					None
				}
			})
			.collect::<Vec<_>>();
		children.sort();
		children
	}

	/// check if [`List`] contains path
	fn contains(&self, other: &Utf8Path) -> bool {
		other.ancestors().any(|p| self == &p)
	}

	/// format [`List`] into [`ratatui::text::Line`] struct for ratatui
	pub fn line(&self, queue: &Queue) -> Line {
		let name = self.path.as_str();

		let underline = Style::default().underlined();
		let accent = ui::style::accent().underlined();
		if let Some(path) = queue.path() {
			if self == &path {
				ui::widgets::line(name, accent.bold())
			} else if self.contains(path) {
				ui::widgets::line(name, accent)
			} else {
				ui::widgets::line(name, underline)
			}
		} else {
			ui::widgets::line(name, underline)
		}
	}

	/// if [`List`] contains path, searches recursively until it finds the matching path
	pub fn find(&self, other: &Utf8Path) -> Option<List> {
		if self == &other {
			Some(self.clone())
		} else if self.contains(other) {
			self.children().into_iter().find_map(|child| match child {
				Child::List(list) => list.find(other),
				Child::Mp3(_) => None,
			})
		} else {
			None
		}
	}
}

impl Eq for List {}

impl PartialEq for List {
	fn eq(&self, other: &Self) -> bool {
		self.path.eq(&other.path)
	}
}

impl PartialEq<&Utf8Path> for List {
	fn eq(&self, other: &&Utf8Path) -> bool {
		self.path.eq(other)
	}
}

impl Serialize for List {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.path.as_path().serialize(serializer)
	}
}

impl List {
	/// deserialize Vec of [`List`]
	///
	/// ignores non-existant [`List`] items
	/// and unwraps an `Option` to an empty vec
	pub fn maybe_deserialize<'de, D>(data: D) -> Result<Vec<List>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let paths: Option<Vec<Utf8PathBuf>> = Deserialize::deserialize(data)?;
		let paths = paths.unwrap_or_default();
		let lists = paths.into_iter().flat_map(List::new).collect();
		Ok(lists)
	}
}

#[derive(Debug, Clone, Copy)]
struct ColorWrap(Color);

impl Deref for ColorWrap {
	type Target = Color;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for ColorWrap {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[derive(Debug, Error)]
#[error("couldn't parse color")]
struct ParseColorError;

impl FromStr for ColorWrap {
	type Err = ParseColorError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let col = s.parse::<Color>().map_err(|_| ParseColorError)?;
		Ok(ColorWrap(col))
	}
}

impl Display for ColorWrap {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut color = self.0.to_string();
		color.make_ascii_lowercase();
		f.write_str(&color)
	}
}

impl Serialize for ColorWrap {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut repr = self.0.to_string();
		repr.make_ascii_lowercase();
		serializer.serialize_str(&repr)
	}
}

impl<'de> Deserialize<'de> for ColorWrap {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(ColorVis)
	}
}

struct ColorVis;

impl serde::de::Visitor<'_> for ColorVis {
	type Value = ColorWrap;

	fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		fmt.write_str("a color")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		v.parse::<ColorWrap>().map_err(serde::de::Error::custom)
	}
}

/// config file
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	/// amount to increase / decrease volume by in percent
	#[serde(skip_serializing_if = "Option::is_none")]
	vol: Option<u8>,
	/// amount to seek by in tracks in seconds
	#[serde(skip_serializing_if = "Option::is_none")]
	seek: Option<u8>,
	/// ui accent color
	#[serde(skip_serializing_if = "Option::is_none")]
	accent: Option<ColorWrap>,
	/// list of playlists
	#[serde(skip_serializing_if = "Vec::is_empty")]
	#[serde(deserialize_with = "List::maybe_deserialize")]
	#[serde(default)]
	lists: Vec<List>,
}

impl Config {
	/// read from [`CONFIG_PATH`] and init [`Config`] struct
	///
	/// todo gracefully handle malformed json
	pub fn init() -> Result<Self, ConfigError> {
		let file = fs::read_to_string(&*CONFIG_PATH)?;
		let config = serde_json::from_str(&file)?;
		Ok(config)
	}

	/// get reference to [`Config::lists`]
	#[inline]
	pub fn lists(&self) -> &[List] {
		&self.lists
	}

	/// get [`Config::seek`] or unwrap to default value of 5
	#[inline]
	pub fn seek(&self) -> Duration {
		let seek = self.seek.unwrap_or(5);
		Duration::from_secs(u64::from(seek))
	}

	/// get and deref [`Config::color`] to [`ratatui::style::Color`]
	#[inline]
	pub fn accent(&self) -> Option<Color> {
		self.accent.as_deref().copied()
	}

	/// get [`Config::vol`] or unwrap to default value of 5
	#[inline]
	pub fn vol(&self) -> u8 {
		self.vol.unwrap_or(5)
	}
}

#[cfg(test)]
mod test {
	use super::{Child, ColorWrap, ConfigError, List};
	use camino::Utf8PathBuf;
	use std::cmp::Ordering;

	/// create [`List`]
	///
	/// # Errors
	///
	/// errors when the path doesn't exist
	fn list<P: Into<Utf8PathBuf>>(path: P) -> Result<List, ConfigError> {
		let path = path.into();
		List::new(path)
	}

	/// create [`Child::List`]
	fn child<P: Into<Utf8PathBuf>>(path: P) -> Child {
		let path = path.into();
		let list = List { path, parent: None };
		Child::List(list)
	}

	/// create [`Child::Mp3`]
	fn mp3<P: Into<Utf8PathBuf>>(path: P) -> Child {
		let path = path.into();
		Child::Mp3(path)
	}

	#[test]
	fn list_contains() -> color_eyre::Result<()> {
		let mock = list("mock/list 01")?;

		let one = mock.contains("mock/list 01/track 00.mp3".into());
		assert!(one);

		let two = mock.contains("mock/list 01/sub 01".into());
		assert!(two);

		let thr = mock.contains("mock/list 01".into());
		assert!(thr);

		let fou = mock.contains("mock/list 01/sub 02/sub sub/".into());
		assert!(fou);

		let fiv = mock.contains("mock".into());
		assert!(!fiv);

		let six = mock.contains("/".into());
		assert!(!six);

		Ok(())
	}

	#[test]
	fn list_find() -> color_eyre::Result<()> {
		let mock = list("mock/list 01")?;

		let one = Utf8PathBuf::from("mock/list 01");
		let one = mock.find(&one);
		let lis = list("mock/list 01")?;
		assert_eq!(one, Some(lis));

		let two = Utf8PathBuf::from("mock/list 01/sub 01");
		let two = mock.find(&two);
		let lis = list("mock/list 01/sub 01")?;
		assert_eq!(two, Some(lis));

		let thr = Utf8PathBuf::from("mock/list 01/track 01.mp3");
		assert!(thr.exists());
		let thr = mock.find(&thr);
		assert!(thr.is_none());

		let fou = Utf8PathBuf::from("mock");
		let fou = mock.find(&fou);
		assert!(fou.is_none());

		Ok(())
	}

	#[test]
	fn ord() {
		let zer3 = mp3("00");
		let one3 = mp3("01");

		let zerc = child("00");
		let onec = child("01");

		assert_eq!(zer3.cmp(&one3), Ordering::Less);
		assert_eq!(zer3.cmp(&zer3), Ordering::Equal);

		assert_eq!(zerc.cmp(&onec), Ordering::Less);
		assert_eq!(zerc.cmp(&zerc), Ordering::Equal);

		assert_eq!(zer3.cmp(&zerc), Ordering::Greater);
		assert_eq!(one3.cmp(&zerc), Ordering::Greater);

		assert_eq!(zerc.cmp(&zer3), Ordering::Less);
		assert_eq!(zerc.cmp(&one3), Ordering::Less);
	}

	#[test]
	fn case_ord() {
		let one = mp3("a");
		let two = mp3("B");
		let thr = mp3("A");
		let fou = mp3("b");

		assert_eq!(one.cmp(&two), Ordering::Less);
		assert_eq!(two.cmp(&one), Ordering::Greater);
		assert_eq!(thr.cmp(&fou), Ordering::Less);
		assert_eq!(fou.cmp(&thr), Ordering::Greater);

		assert_eq!(one.cmp(&thr), Ordering::Equal);
		assert_eq!(thr.cmp(&one), Ordering::Equal);
		assert_eq!(two.cmp(&fou), Ordering::Equal);
		assert_eq!(fou.cmp(&two), Ordering::Equal);
	}

	#[test]
	fn unicode_ord() {
		let one = mp3("ä");
		let two = mp3("Ü");
		let thr = mp3("Ä");
		let fou = mp3("ü");

		assert_eq!(one.cmp(&two), Ordering::Less);
		assert_eq!(two.cmp(&one), Ordering::Greater);
		assert_eq!(thr.cmp(&fou), Ordering::Less);
		assert_eq!(fou.cmp(&thr), Ordering::Greater);

		assert_eq!(one.cmp(&thr), Ordering::Equal);
		assert_eq!(thr.cmp(&one), Ordering::Equal);
		assert_eq!(two.cmp(&fou), Ordering::Equal);
		assert_eq!(fou.cmp(&two), Ordering::Equal);
	}

	#[test]
	fn children() -> color_eyre::Result<()> {
		let mock = list("mock/list 01")?;
		let comp = vec![
			child("mock/list 01/sub 01"),
			child("mock/list 01/sub 02"),
			mp3("mock/list 01/track 00.mp3"),
			mp3("mock/list 01/track 01.mp3"),
		];

		let children = mock.children();
		assert_eq!(children, comp);

		Ok(())
	}

	#[test]
	fn parse_col() -> color_eyre::Result<()> {
		assert!("cyan".parse::<ColorWrap>().is_ok());
		assert!("light-gray".parse::<ColorWrap>().is_ok());
		assert!("Blue".parse::<ColorWrap>().is_ok());
		assert!("MAGENTA".parse::<ColorWrap>().is_ok());
		assert!("BRIGHTCYAN".parse::<ColorWrap>().is_ok());
		assert!("LIGHT_red".parse::<ColorWrap>().is_ok());

		assert!("#008080".parse::<ColorWrap>().is_ok());
		assert!("10".parse::<ColorWrap>().is_ok());

		assert!("none".parse::<ColorWrap>().is_err());
		assert!("".parse::<ColorWrap>().is_err());

		Ok(())
	}
}
