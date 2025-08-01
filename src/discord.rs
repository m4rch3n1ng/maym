use crate::state::State;
use discord_rich_presence::{
	DiscordIpc, DiscordIpcClient,
	activity::{Activity, ActivityType, Assets, Timestamps},
};
use std::{
	fmt::Debug,
	sync::mpsc::Sender,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

/// amt of time to wait before a retry
const WAIT: Duration = Duration::from_secs(30);

const CLIENT_ID: &str = "1170754365619982346";

#[derive(Debug)]
pub struct Discord(Sender<State>);

impl Discord {
	pub fn new() -> Self {
		let (tx, rx) = std::sync::mpsc::channel::<State>();

		std::thread::spawn(move || {
			let mut discord = DiscordState::connect();
			for state in rx {
				discord.state(&state);
			}
		});

		Discord(tx)
	}

	pub fn state(&self, state: State) {
		let _ = self.0.send(state);
	}
}

enum DiscordState {
	Connected(DiscordIpcClient),
	Disconnected(SystemTime),
}

impl DiscordState {
	fn connect() -> DiscordState {
		let mut discord = DiscordIpcClient::new(CLIENT_ID).expect("should never panic");
		if discord.connect().is_ok() {
			DiscordState::Connected(discord)
		} else {
			let now = SystemTime::now();
			DiscordState::Disconnected(now)
		}
	}

	fn client(&mut self) -> Option<&mut DiscordIpcClient> {
		if let DiscordState::Disconnected(time) = self
			&& time.elapsed().unwrap_or(WAIT) >= WAIT
		{
			*self = DiscordState::connect();
		}

		match self {
			DiscordState::Connected(discord) => Some(discord),
			DiscordState::Disconnected(_) => None,
		}
	}

	fn state(&mut self, state: &State) {
		let Some(discord) = self.client() else { return };
		let res = if let Some(activity) = activity(state) {
			discord.set_activity(activity)
		} else {
			discord.clear_activity()
		};

		if res.is_err() {
			let now = SystemTime::now();
			*self = DiscordState::Disconnected(now);
		}
	}
}

impl Debug for DiscordState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			DiscordState::Connected(_) => f.debug_tuple("Connected").field(&..).finish(),
			DiscordState::Disconnected(_) => f.write_str("Disconnected"),
		}
	}
}

impl Drop for DiscordState {
	fn drop(&mut self) {
		if let DiscordState::Connected(discord) = self {
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

fn activity(state: &State) -> Option<Activity<'_>> {
	if let Some(track) = state.track.as_ref() {
		let title = track.title().unwrap_or("unknown title");
		let artist = track.artist().unwrap_or("unknown artist");

		let mut activity = Activity::new()
			.activity_type(ActivityType::Listening)
			.details(title)
			.state(artist);

		let assets = Assets::new().large_image("icon");
		if state.paused {
			if let Some((elapsed, duration)) = state.elapsed_duration() {
				let (start, _) = timestamps(duration, elapsed);
				let timestamps = Timestamps::new().start(start);
				activity = activity.timestamps(timestamps);
			}

			let assets = assets.small_image("paused").small_text("paused");
			Some(activity.assets(assets))
		} else if let Some((elapsed, duration)) = state.elapsed_duration() {
			let (start, end) = timestamps(duration, elapsed);
			let timestamps = Timestamps::new().start(start).end(end);

			let assets = assets.small_image("play").small_text("play");
			Some(activity.assets(assets).timestamps(timestamps))
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
