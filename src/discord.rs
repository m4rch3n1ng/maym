use crate::state::State;
use discord_rich_presence::{
	activity::{Activity, Assets, Timestamps},
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

	fn activity<'s>(&self, state: &'s State) -> Option<Activity<'s>> {
		if let Some(track) = state.track.as_ref() {
			let title = track.title().unwrap_or("unknown title");
			let artist = track.artist().unwrap_or("unknown artist");

			let activity = Activity::new().details(title).state(artist);

			let asset = Assets::new().large_image("icon");
			let activity = activity.assets(asset);

			if state.paused {
				Some(activity)
			} else if let Some((elapsed, duration)) = state.elapsed_duration() {
				let (start, end) = Self::timestamps(duration, elapsed);
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

	pub fn state(&mut self, state: &State) {
		let activity = self.activity(state);
		if let Some(activity) = activity {
			self.0.set_activity(activity).unwrap();
		} else {
			self.0.clear_activity().unwrap();
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

#[cfg(test)]
mod test {
	use super::Discord;
	use std::time::Duration;

	#[test]
	fn time() {
		let duration = Duration::from_secs(100);
		let elapsed = Duration::ZERO;

		let (start, end) = Discord::timestamps(duration, elapsed);
		assert_eq!(end - start, duration.as_millis() as i64);

		let duration = Duration::from_secs(50);
		let elapsed = Duration::from_secs(25);

		let (start, end) = Discord::timestamps(duration, elapsed);
		assert_eq!(end - start, duration.as_millis() as i64);
	}
}
