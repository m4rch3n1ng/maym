use crate::state::State;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/*

let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
let end = now + 4 * 60 * 1000;
let timestamps = activity::Timestamps::new().start(0).end(end);
let payload = activity::Activity::new().details("current track").state("fuck").timestamps(timestamps);
client.set_activity(payload).unwrap(); */

pub struct Discord(DiscordIpcClient);

impl Discord {
	pub fn new() -> Self {
		let client = DiscordIpcClient::new("1170754365619982346").unwrap();
		Discord(client)
	}

	pub fn connect(&mut self) {
		self.0.connect().unwrap();
	}

	pub fn state(&mut self, state: &State) {
		let track = state.track.as_ref().unwrap();
		let title = track.title().unwrap();
		let artist = track.artist().unwrap();

		let activity = activity::Activity::new().details(title).state(artist);
		let activity = if state.paused {
			activity
		} else {
			let duration = state.duration().unwrap();
			let now = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap()
				.as_millis() as u64;
			let now = Duration::from_millis(now);
			let end = now + duration;
			let end = end.as_millis() as i64;
			let timestamps = activity::Timestamps::new().start(0).end(end);
			activity.timestamps(timestamps)
		};

		self.0.set_activity(activity).unwrap();
	}
}
