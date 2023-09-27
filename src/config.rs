use camino::Utf8PathBuf;
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

#[derive(Debug, PartialEq, Eq)]
pub enum Child {
	List(List),
	Mp3(Utf8PathBuf),
}

impl Child {
	fn name(&self) -> &str {
		match *self {
			Child::List(ref list) => list.path.file_name().unwrap(),
			Child::Mp3(ref path) => path.file_name().unwrap(),
		}
	}

	pub fn list(&self) -> Option<&List> {
		match self {
			Child::List(list) => Some(list),
			Child::Mp3(_) => None,
		}
	}

	pub fn line(&self) -> Line {
		let name = self.name();
		match *self {
			Child::List(_) => {
				let line = format!("{}/", name);
				Line::styled(line, Style::default().underlined())
			}
			Child::Mp3(_) => Line::from(name),
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

#[derive(Debug, Clone)]
pub struct List {
	path: Utf8PathBuf,
	parent: Option<Rc<List>>,
}

impl List {
	fn path(path: Utf8PathBuf) -> Result<Self, ConfigError> {
		if path.exists() {
			let list = List { path, parent: None };
			Ok(list)
		} else {
			Err(ConfigError::ListDoesntExist(path))
		}
	}

	pub fn with_parent(path: Utf8PathBuf, parent: Rc<List>) -> Result<Self, ConfigError> {
		if path.exists() {
			let list = List {
				path,
				parent: Some(parent),
			};
			Ok(list)
		} else {
			Err(ConfigError::ListDoesntExist(path))
		}
	}

	pub fn parent(&self) -> Option<List> {
		self.parent.as_ref().map(|rc| (**rc).clone())
	}

	// todo error handling
	pub fn children(&self) -> Vec<Child> {
		let read = fs::read_dir(&self.path).unwrap();
		let mut children = read
			.flatten()
			// todo display non utf8
			.map(|entry| entry.path())
			.flat_map(Utf8PathBuf::try_from)
			.filter_map(|path| {
				if path.is_dir() {
					let parent = Rc::new(self.clone());
					let list = List::with_parent(path, parent).unwrap();
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
}

impl Eq for List {}

impl PartialEq for List {
	fn eq(&self, other: &Self) -> bool {
		self.path.eq(&other.path)
	}
}

impl TryFrom<Utf8PathBuf> for List {
	type Error = ConfigError;
	fn try_from(path: Utf8PathBuf) -> Result<Self, Self::Error> {
		if path.exists() {
			let list = List { path, parent: None };
			Ok(list)
		} else {
			Err(ConfigError::ListDoesntExist(path))
		}
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
		let track = lists_or.map(|lists| lists.into_iter().flat_map(List::path).collect());
		Ok(track)
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	#[serde(
		skip_serializing_if = "Option::is_none",
		deserialize_with = "List::maybe_deserialize"
	)]
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
