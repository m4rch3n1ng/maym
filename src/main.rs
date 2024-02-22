use self::{
	config::Config,
	player::Player,
	queue::{Queue, QueueError},
	state::{State, StateError},
	ui::{Popups, Ui},
};
use color_eyre::eyre::Context;
use crossterm::{
	cursor,
	event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind},
	execute, terminal,
};
use ratatui::{
	prelude::{Backend, CrosstermBackend},
	Terminal,
};
use std::{
	io,
	time::{Duration, Instant},
};
use thiserror::Error;

mod config;
mod player;
mod queue;
mod state;
mod ui;

#[derive(Debug, Error)]
enum MusicError {
	#[error("quit")]
	Quit,
	#[error("io error")]
	IoError(#[from] std::io::Error),
	#[error("queue error")]
	QueueError(#[from] QueueError),
	#[error("state error")]
	StateError(#[from] StateError),
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
	pub fn new() -> color_eyre::Result<Self> {
		let config = Config::init()?;
		let state = State::init();
		let queue = Queue::state(&state)?;

		let mut player = Player::new()?;
		player.state(&queue, &state)?;

		let ui = Ui::new(&queue, &config);

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

	pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), MusicError> {
		let mut last = Instant::now();
		let mut skip_done = false;
		let mut ticks = 0;

		loop {
			terminal.draw(|f| self.ui.draw(f, &self.state, &self.queue))?;

			let timeout = self.tick.saturating_sub(last.elapsed());
			if event::poll(timeout)? {
				match event::read()? {
					Event::Key(key) if key.kind == KeyEventKind::Press => {
						self.handle(key, &mut skip_done)?;
					}
					Event::Mouse(mouse) => match mouse.kind {
						MouseEventKind::ScrollDown => self.ui.down(),
						MouseEventKind::ScrollUp => self.ui.up(),
						_ => {}
					},
					_ => {}
				}
			}

			if last.elapsed() >= self.tick {
				self.state.tick(&self.player, &self.queue, &mut self.ui);
				if !skip_done {
					self.queue.done(&mut self.player, &self.state)?;
				} else {
					skip_done = false;
				}

				last = Instant::now();

				// todo amt
				if ticks >= 10 {
					self.state.write()?;
					ticks = 0;
				} else {
					ticks += 1;
				}
			}
		}
	}

	fn handle(&mut self, key: KeyEvent, skip_done: &mut bool) -> Result<(), MusicError> {
		let seek = self.config.seek();
		let vol = self.config.vol();

		match (key.code, key.modifiers) {
			// global
			(KeyCode::Char('q' | 'Q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
				return Err(MusicError::Quit)
			}
			// player
			(KeyCode::Char(' '), KeyModifiers::ALT) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
				self.player.toggle();
			}
			(KeyCode::Char('m'), KeyModifiers::NONE) => self.player.mute(),
			(KeyCode::Up, KeyModifiers::SHIFT) => self.player.i_vol(vol),
			(KeyCode::Down, KeyModifiers::SHIFT) => self.player.d_vol(vol),
			// queue
			(KeyCode::Right, KeyModifiers::SHIFT) => {
				let _ = self.queue.next(&mut self.player);
				*skip_done = true;
			}
			(KeyCode::Left, KeyModifiers::SHIFT) => {
				self.queue.last(&mut self.player);
				*skip_done = true;
			}
			(KeyCode::Char('0'), KeyModifiers::NONE) => {
				self.queue.restart(&mut self.player);
			}
			(KeyCode::Char('s'), KeyModifiers::NONE) => {
				self.queue.shuffle();
			}
			// ui
			(KeyCode::Esc, KeyModifiers::NONE) => self.ui.esc(),
			(KeyCode::Char('i'), KeyModifiers::NONE) => self.ui.tags(),
			(KeyCode::Char('y'), KeyModifiers::NONE) => self.ui.lyrics(),
			(KeyCode::Char('t'), KeyModifiers::NONE) => self.ui.tracks(),
			(KeyCode::Char('l'), KeyModifiers::NONE) => self.ui.lists(),
			(KeyCode::Down, KeyModifiers::NONE) => self.ui.down(),
			(KeyCode::Up, KeyModifiers::NONE) => self.ui.up(),
			(KeyCode::PageDown, KeyModifiers::NONE) => self.ui.pg_down(),
			(KeyCode::PageUp, KeyModifiers::NONE) => self.ui.pg_up(),
			(KeyCode::Home, KeyModifiers::NONE) => self.ui.home(),
			(KeyCode::End, KeyModifiers::NONE) => self.ui.end(),
			(KeyCode::Backspace, KeyModifiers::NONE) => self.ui.backspace(),
			(KeyCode::Enter, KeyModifiers::NONE) => {
				self.ui.enter(&mut self.player, &mut self.queue)?;
				*skip_done = true;
			}
			// ctx
			(KeyCode::Char(' '), KeyModifiers::NONE) => match self.ui.popup {
				Some(Popups::Lists | Popups::Tracks) => {
					self.ui.space(&mut self.player, &mut self.queue)?;
					*skip_done = true;
				}
				_ => self.player.toggle(),
			},
			(KeyCode::Right, KeyModifiers::NONE) => {
				if self.ui.is_popup() {
					self.ui.right();
				} else {
					self.queue.seek_i(&mut self.player, &self.state, seek);
				}
			}
			(KeyCode::Left, KeyModifiers::NONE) => {
				if self.ui.is_popup() {
					self.ui.left();
				} else {
					self.queue.seek_d(&mut self.player, &self.state, seek);
				}
			}
			// ignore
			_ => {}
		}

		Ok(())
	}

	pub fn start(&mut self) -> color_eyre::Result<()> {
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
		match result {
			Err(MusicError::Quit) | Ok(()) => Ok(()),
			Err(err) => Err(color_eyre::Report::from(err)),
		}
	}
}

impl Drop for Application {
	fn drop(&mut self) {
		let mut stdout = io::stdout();

		let _ = terminal::disable_raw_mode();
		let _ = execute!(
			stdout,
			terminal::LeaveAlternateScreen,
			event::DisableMouseCapture
		);

		let _ = execute!(stdout, cursor::Show);
	}
}

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	let mut app = Application::new().wrap_err("music error")?;
	app.start().wrap_err("music error")?;

	Ok(())
}
