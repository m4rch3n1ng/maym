use crate::player::Player;

#[derive(Debug)]
pub struct State {
	volume: f64,
	paused: bool,
	muted: bool,
	remaining: Option<f64>,
	duration: Option<f64>,
}

impl State {
	pub fn new(player: &Player) -> Self {
		let volume = player.volume();
		let paused = player.paused();
		let muted = player.muted();
		let remaining = player.remaining();
		let duration = player.duration();

		State {
			volume,
			paused,
			muted,
			remaining,
			duration,
		}
	}

	pub fn tick(&mut self, player: &Player) {
		self.volume = player.volume();
		self.paused = player.paused();
		self.muted = player.muted();
		self.remaining = player.remaining();
		self.duration = player.duration();
	}
}
