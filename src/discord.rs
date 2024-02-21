use crate::state::State;
use discord_rich_presence::{
	activity::{Activity, Assets, Timestamps},
	DiscordIpc, DiscordIpcClient,
};
use std::{
	fmt::Debug,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

/// amt of time to wait before a retry
///
/// todo maybe don't wait,
/// but retry on track change
const WAIT: Duration = Duration::from_secs(30);

const CLIENT_ID: &str = "1170754365619982346";

#[derive(Default)]
pub enum Discord {
	Connect(DiscordIpcClient),
	NotConnect(SystemTime),
	#[default]
	NotYet,
}

impl Discord {
	pub fn connect(&mut self) {
		let mut discord = DiscordIpcClient::new(CLIENT_ID).expect("should never panic");
		let discord = if discord.connect().is_ok() {
			Discord::Connect(discord)
		} else {
			let now = SystemTime::now();
			Discord::NotConnect(now)
		};

		*self = discord;
	}

	fn revive(&mut self) {
		match self {
			Discord::Connect(_) => (),
			Discord::NotConnect(prev) => {
				let now = SystemTime::now();
				let diff = now.duration_since(*prev).unwrap_or(WAIT);

				if diff >= WAIT {
					self.connect();
				}
			}
			Discord::NotYet => self.connect(),
		}
	}

	fn client(&mut self) -> Option<&mut DiscordIpcClient> {
		self.revive();
		match self {
			Discord::Connect(discord) => Some(discord),
			Discord::NotConnect(_) => None,
			Discord::NotYet => unreachable!(),
		}
	}

	pub fn state(&mut self, state: &State) {
		let Some(discord) = self.client() else { return };

		let activity = activity(state);
		let res = if let Some(activity) = activity {
			discord.set_activity(activity)
		} else {
			discord.clear_activity()
		};

		if res.is_err() {
			let now = SystemTime::now();
			let now = Discord::NotConnect(now);
			*self = now;
		}
	}
}

impl Debug for Discord {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Discord::Connect(_) => f.debug_tuple("Discord").field(&..).finish(),
			Discord::NotConnect(_) => f.write_str("Invalid"),
			Discord::NotYet => f.write_str("NotYet"),
		}
	}
}

impl Drop for Discord {
	fn drop(&mut self) {
		if let Discord::Connect(discord) = self {
			let _ = discord.clear_activity();
		}
	}
}

fn timestamps(duration: Duration, elapsed: Duration) -> (i64, i64) {
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("UNIX_EPOCH should always be in the past");

	let start = now - elapsed;
	let end = start + duration;

	let start = start.as_millis() as i64;
	let end = end.as_millis() as i64;

	(start, end)
}

fn activity(state: &State) -> Option<Activity> {
	if let Some(track) = state.track.as_ref() {
		let title = track.title().unwrap_or("unknown title");
		let artist = track.artist().unwrap_or("unknown artist");

		let activity = Activity::new().details(title).state(artist);

		let asset = Assets::new().large_image("icon");
		let activity = activity.assets(asset);

		if state.paused {
			Some(activity)
		} else if let Some((elapsed, duration)) = state.elapsed_duration() {
			let (start, end) = timestamps(duration, elapsed);
			let timestamps = Timestamps::new().start(start).end(end);

			let activity = activity.timestamps(timestamps);
			Some(activity)
		} else {
			None
		}
	} else {
		None
	}
}

#[cfg(test)]
mod test {
	use super::timestamps;
	use std::time::Duration;

	#[test]
	fn time() {
		let duration = Duration::from_secs(100);
		let elapsed = Duration::ZERO;

		let (start, end) = timestamps(duration, elapsed);
		assert_eq!(end - start, duration.as_millis() as i64);

		let duration = Duration::from_secs(50);
		let elapsed = Duration::from_secs(25);

		let (start, end) = timestamps(duration, elapsed);
		assert_eq!(end - start, duration.as_millis() as i64);
	}
}
