use serde::{Deserialize, Serialize};
use std::fs;

const PATH: &str = "/home/may/.config/m4rch/player/config.json";

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	seek: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	vol: Option<u64>,
}

impl Config {
	pub fn init() -> Self {
		let file = fs::read_to_string(PATH).unwrap();
		serde_json::from_str(&file).unwrap()
	}

	pub fn seek(&self) -> u64 {
		self.seek.unwrap_or(5)
	}

	pub fn vol(&self) -> u64 {
		self.vol.unwrap_or(5)
	}
}
