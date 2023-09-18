use self::{player::Player, state::State};
use config::{Config, ConfigError};
use crossterm::{
	event::{self, Event, KeyCode, KeyModifiers},
	execute, terminal,
};
use queue::{Queue, QueueError};
use ratatui::{
	prelude::{Backend, CrosstermBackend},
	Terminal,
};
use state::StateError;
use std::{
	io,
	time::{Duration, Instant},
};
use thiserror::Error;
use ui::Ui;

mod config;
mod player;
mod queue;
mod state;
mod ui;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum MayError {
	#[error("mpv error")]
	MpvError(#[from] mpv::Error),
	#[error("state error")]
	StateError(#[from] StateError),
	#[error("config error")]
	ConfigError(#[from] ConfigError),
	#[error("queue error")]
	QueueError(#[from] QueueError),
	#[error("io error")]
	IoError(#[from] std::io::Error),
}

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
	pub fn new() -> Result<Self, MayError> {
		let config = Config::init()?;
		let state = State::init()?;
		let queue = Queue::state(&state);

		let mut player = Player::new()?;
		player.state(&queue, &state)?;

		let ui = Ui::default();

		let tick = Duration::from_millis(100);

		let app = Application {
			player,
			config,
			state,
			queue,
			ui,
			tick,
		};
		Ok(app)
	}

	pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), MayError> {
		let mut last = Instant::now();
		let mut ticks = 0;

		let seek = self.config.seek();
		let vol = self.config.vol();

		loop {
			terminal.draw(|f| self.ui.draw(f, &self.state))?;

			let timeout = self.tick.saturating_sub(last.elapsed());
			if event::poll(timeout)? {
				// todo check if press
				if let Event::Key(key) = event::read()? {
					match (key.code, key.modifiers) {
						// global
						(KeyCode::Char('q' | 'Q'), _) => return Ok(()),
						// player
						(KeyCode::Char(' ' | 'k'), KeyModifiers::NONE) => self.player.toggle(),
						(KeyCode::Char('m'), KeyModifiers::NONE) => self.player.mute(),
						(KeyCode::Up, KeyModifiers::SHIFT) => self.player.i_vol(vol),
						(KeyCode::Down, KeyModifiers::SHIFT) => self.player.d_vol(vol),
						// queue
						(KeyCode::Right, KeyModifiers::SHIFT) => {
							// todo that error can probably be ignored
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
					self.state.write().unwrap();
					ticks = 0;
				} else {
					ticks += 1;
				}
			}
		}
	}

	pub fn start(&mut self) -> Result<(), MayError> {
		let mut stdout = io::stdout();

		terminal::enable_raw_mode()?;
		execute!(
			stdout,
			terminal::EnterAlternateScreen,
			event::EnableMouseCapture
		)?;

		let backend = CrosstermBackend::new(&stdout);
		let mut terminal = Terminal::new(backend)?;

		let result = self.run(&mut terminal);

		let _ = terminal::disable_raw_mode();
		let _ = execute!(
			terminal.backend_mut(),
			terminal::LeaveAlternateScreen,
			event::DisableMouseCapture
		);

		let _ = terminal.show_cursor();

		result
	}
}

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	let mut app = Application::new()?;
	app.start()?;

	Ok(())
}
