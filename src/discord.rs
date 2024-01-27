use crate::state::State;
use discord_rich_presence::{
	activity::{Activity, Timestamps},
	DiscordIpc, DiscordIpcClient,
};
use std::{
	fmt::Debug,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct Discord(DiscordIpcClient);

impl Discord {
	pub fn new() -> Self {
		let client = DiscordIpcClient::new("1170754365619982346").unwrap();
		Discord(client)
	}

	pub fn connect(&mut self) {
		self.0.connect().unwrap();
	}

	fn start_end(&self, duration: Duration, elapsed: Duration) -> (i64, i64) {
		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("UNIX_EPOCH should always be in the past");

		let start = now - elapsed;
		let end = start + duration;

		let start = start.as_millis() as i64;
		let end = end.as_millis() as i64;

		(start, end)
	}

	pub fn state(&mut self, state: &State) {
		let track = state.track.as_ref().unwrap();
		let title = track.title().unwrap();
		let artist = track.artist().unwrap();

		let activity = Activity::new().details(title).state(artist);
		let activity = if state.paused {
			Some(activity)
		} else if let Some((elapsed, duration)) = state.elapsed_duration() {
			let (start, end) = self.start_end(duration, elapsed);
			let timestamps = Timestamps::new().start(start).end(end);

			let activity = activity.timestamps(timestamps);
			Some(activity)
		} else {
			None
		};

		if let Some(activity) = activity {
			self.0.set_activity(activity).unwrap();
		}
	}
}

impl Debug for Discord {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Discord").finish_non_exhaustive()
	}
}

impl Drop for Discord {
	fn drop(&mut self) {
		let _ = self.0.clear_activity();
	}
}
