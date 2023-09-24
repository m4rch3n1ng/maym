use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};
use thiserror::Error;

const PATH: &str = "/home/may/.config/m4rch/player/config.json";

#[derive(Debug, Error)]
pub enum ConfigError {
	#[error("io errors")]
	IoError(#[from] std::io::Error),
	#[error("serde error")]
	SerdeJsonError(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	seek: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	vol: Option<u64>,
}

impl Config {
	pub fn init() -> Result<Self, ConfigError> {
		let file = fs::read_to_string(PATH)?;
		let config = serde_json::from_str(&file)?;
		Ok(config)
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
