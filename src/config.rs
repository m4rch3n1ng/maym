use crate::{
	queue::{Queue, Track},
	ui::utils,
};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use ratatui::{
	style::{Style, Stylize},
	text::Line,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fs, rc::Rc, time::Duration};
use thiserror::Error;

static PATH: Lazy<Utf8PathBuf> =
	Lazy::new(|| Utf8PathBuf::from("/home/may/.config/m4rch/player/config.json"));

#[derive(Debug, Error)]
pub enum ConfigError {
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("serde error")]
	SerdeJsonError(#[from] serde_json::Error),
	#[error("list {0:?} doesn't exist")]
	ListDoesntExist(Utf8PathBuf),
}

/// [`Child`] struct of [`List`]
///
/// created via [`List::children`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Child {
	List(List),
	Mp3(Utf8PathBuf),
}

impl Child {
	/// return name of child
	fn name(&self) -> &str {
		match *self {
			Child::List(ref list) => list.path.file_name().unwrap(),
			Child::Mp3(ref path) => path.file_name().unwrap(),
		}
	}

	/// returns list if child is [`Child::List`].
	pub fn list(&self) -> Option<&List> {
		match self {
			Child::List(list) => Some(list),
			Child::Mp3(_) => None,
		}
	}

	/// check if [`Child::List`] contains path, or if [`Child::Mp3`] is path
	fn contains(&self, other: &Utf8Path) -> bool {
		match &self {
			Child::List(list) => other.ancestors().any(|p| list == &p),
			Child::Mp3(path) => path == other,
		}
	}

	pub fn line(&self, queue: &Queue) -> Line {
		let name = self.name();
		match *self {
			Child::List(ref list) => {
				let fmt = format!("{}/", name);
				let underline = Style::default().underlined();
				let accent = utils::style::accent().underlined();
				if let Some(path) = queue.path().map(AsRef::as_ref) {
					if list == &path {
						Line::styled(fmt, accent.bold())
					} else if self.contains(path) {
						Line::styled(fmt, accent)
					} else {
						Line::styled(fmt, underline)
					}
				} else {
					Line::styled(fmt, underline)
				}
			}
			Child::Mp3(ref path) => {
				if let Some(track) = queue.track() {
					if track == path {
						Line::styled(name, utils::style::accent().bold())
					} else {
						Line::from(name)
					}
				} else {
					Line::from(name)
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
			(Child::List(l1), Child::List(l2)) => l1
				.path
				.as_str()
				.to_lowercase()
				.cmp(&l2.path.as_str().to_lowercase()),
			(Child::Mp3(p1), Child::Mp3(p2)) => {
				p1.as_str().to_lowercase().cmp(&p2.as_str().to_lowercase())
			}
			(&Child::List(_), &Child::Mp3(_)) => std::cmp::Ordering::Less,
			(&Child::Mp3(_), &Child::List(_)) => std::cmp::Ordering::Greater,
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
	pub path: Utf8PathBuf,
	parent: Option<Rc<List>>,
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
			let parent = Rc::new(parent);
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
	pub fn parent(&self) -> Option<List> {
		self.parent.as_ref().map(|rc| (**rc).clone())
	}

	// todo error handling
	/// reads files in [`List`] and returns a vec of [`Child`] structs
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
				} else if path.extension().map_or(false, |ext| ext == "mp3") {
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

	/// format [`List`] into [`Line`] struct for ratatui
	pub fn line(&self, queue: &Queue) -> Line {
		let name = self.path.as_str();

		let underline = Style::default().underlined();
		let accent = utils::style::accent().underlined();
		if let Some(path) = queue.path() {
			if self == &path.as_path() {
				Line::styled(name, accent.bold())
			} else if self.contains(path) {
				Line::styled(name, accent)
			} else {
				Line::styled(name, underline)
			}
		} else {
			Line::styled(name, underline)
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
	pub fn maybe_deserialize<'de, D>(data: D) -> Result<Option<Vec<List>>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let lists_or: Option<Vec<Utf8PathBuf>> = Deserialize::deserialize(data)?;
		let track = lists_or.map(|lists| lists.into_iter().flat_map(List::new).collect());
		Ok(track)
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(deserialize_with = "List::maybe_deserialize")]
	lists: Option<Vec<List>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	seek: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	vol: Option<u64>,
}

impl Config {
	pub fn init() -> Result<Self, ConfigError> {
		let file = fs::read_to_string(&*PATH)?;
		let config = serde_json::from_str(&file)?;
		Ok(config)
	}

	#[inline]
	pub fn lists(&self) -> &[List] {
		self.lists.as_deref().unwrap_or_default()
	}

	#[inline]
	pub fn seek(&self) -> Duration {
		let seek = self.seek.unwrap_or(5);
		Duration::from_secs(seek)
	}

	#[inline]
	pub fn vol(&self) -> u64 {
		self.vol.unwrap_or(5)
	}
}

#[cfg(test)]
mod test {
	use super::{Child, List};
	use camino::Utf8PathBuf;

	fn list<P: Into<Utf8PathBuf>>(path: P) -> List {
		let path = path.into();
		List { path, parent: None }
	}

	fn child<P: Into<Utf8PathBuf>>(path: P) -> Child {
		let list = list(path);
		Child::List(list)
	}

	fn mp3<P: Into<Utf8PathBuf>>(path: P) -> Child {
		let path = path.into();
		Child::Mp3(path)
	}

	#[test]
	fn list_contains() {
		let mock = list("/path/test");

		let one = mock.contains("/path/test".into());
		assert!(one);

		let two = mock.contains("/path/test/other".into());
		assert!(two);

		let thr = mock.contains("/path/test/other/".into());
		assert!(thr);

		let fou = mock.contains("/path/test/other/more".into());
		assert!(fou);

		let fiv = mock.contains("/path".into());
		assert!(!fiv);

		let six = mock.contains("/test".into());
		assert!(!six)
	}

	#[test]
	fn child_list_contains() {
		let list = child("/list/test");

		let one = list.contains("/list/test".into());
		assert!(one);

		let two = list.contains("/list/test/other".into());
		assert!(two);

		let thr = list.contains("/list".into());
		assert!(!thr);

		let fou = list.contains("/test".into());
		assert!(!fou);
	}

	#[test]
	fn child_mp3_contains() {
		let mp3 = mp3("/mp3/test");

		let one = mp3.contains("/mp3/test".into());
		assert!(one);

		let two = mp3.contains("/mp3/test/other".into());
		assert!(!two);

		let thr = mp3.contains("/mp3".into());
		assert!(!thr);

		let fou = mp3.contains("/mp4".into());
		assert!(!fou);
	}
}
