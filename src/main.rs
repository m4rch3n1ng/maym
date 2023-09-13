use self::{player::Player, state::State};
use config::Config;
use crossterm::{
	event::{self, Event, KeyCode, KeyModifiers},
	execute, terminal,
};
use queue::Queue;
use ratatui::{
	prelude::{Backend, CrosstermBackend},
	Terminal,
};
use std::{
	io,
	time::{Duration, Instant},
};
use ui::Ui;

mod config;
mod player;
mod queue;
mod state;
mod ui;

#[derive(Debug)]
struct Application {
	pub player: Player,
	pub config: Config,
	pub state: State,
	pub queue: Queue,
	pub ui: Ui,
	tick: Duration,
}

impl Application {
	pub fn new() -> Self {
		let config = Config::init();
		let state = State::init();
		let queue = Queue::state(&state);

		let mut player = Player::new();
		player.state(&queue, &state);

		let ui = Ui::default();

		let tick = Duration::from_millis(100);

		Application {
			player,
			config,
			state,
			queue,
			ui,
			tick,
		}
	}

	pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) {
		let mut last = Instant::now();
		let mut ticks = 0;

		let seek = self.config.seek();
		let vol = self.config.vol();

		loop {
			terminal.draw(|f| self.ui.draw(f, &self.state)).unwrap();

			let timeout = self.tick.saturating_sub(last.elapsed());
			if event::poll(timeout).unwrap() {
				// todo check if press
				if let Event::Key(key) = event::read().unwrap() {
					match (key.code, key.modifiers) {
						// global
						(KeyCode::Char('q' | 'Q'), _) => return,
						// player
						(KeyCode::Char(' ' | 'k'), KeyModifiers::NONE) => self.player.toggle(),
						(KeyCode::Char('m'), KeyModifiers::NONE) => self.player.mute(),
						(KeyCode::Up, KeyModifiers::SHIFT) => self.player.i_vol(vol),
						(KeyCode::Down, KeyModifiers::SHIFT) => self.player.d_vol(vol),
						// queue
						(KeyCode::Right, KeyModifiers::SHIFT) => {
							self.queue.next(&mut self.player).unwrap();
							// todo more sophisticated solution
							last = Instant::now();
						}
						(KeyCode::Left, KeyModifiers::SHIFT) => {
							self.queue.last(&mut self.player);
							// todo more sophisticated solution
							last = Instant::now();
						}
						(KeyCode::Char('0'), KeyModifiers::NONE) => {
							self.queue.restart(&mut self.player);
						}
						(KeyCode::Left, KeyModifiers::NONE) => {
							self.queue.seek_d(&mut self.player, &self.state, seek);
						}
						(KeyCode::Right, KeyModifiers::NONE) => {
							self.queue.seek_i(&mut self.player, &self.state, seek);
						}
						// ignore
						_ => {}
					}
				}
			}

			if last.elapsed() >= self.tick {
				self.state.tick(&self.player, &self.queue);
				self.queue.done(&mut self.player, &self.state);

				last = Instant::now();

				// todo amt
				if ticks >= 10 {
					self.state.write();
					ticks = 0;
				} else {
					ticks += 1;
				}
			}
		}
	}

	pub fn start(&mut self) {
		terminal::enable_raw_mode().unwrap();

		let mut stdout = io::stdout();
		execute!(
			stdout,
			terminal::EnterAlternateScreen,
			event::EnableMouseCapture
		)
		.unwrap();

		let backend = CrosstermBackend::new(&stdout);
		let mut terminal = Terminal::new(backend).unwrap();

		self.run(&mut terminal);

		terminal::disable_raw_mode().unwrap();
		execute!(
			terminal.backend_mut(),
			terminal::LeaveAlternateScreen,
			event::DisableMouseCapture
		)
		.unwrap();

		terminal.show_cursor().unwrap();
	}
}

fn main() {
	color_eyre::install().unwrap();

	let mut app = Application::new();

	app.start();
}
