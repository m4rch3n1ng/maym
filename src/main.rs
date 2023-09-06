use crossterm::{
	event::{self, Event, KeyCode, KeyModifiers},
	execute, terminal,
};
use std::{
	io,
	time::{Duration, Instant},
};

use player::Player;
use ratatui::{
	prelude::{Backend, CrosstermBackend},
	Terminal,
};
use state::State;
use tui::Tui;

mod player;
mod state;
mod tui;

#[derive(Debug)]
struct Application {
	pub player: Player,
	pub tui: Tui,
	pub state: State,
	tick: Duration,
}

impl Application {
	pub fn new() -> Self {
		let player = Player::new();
		let tui = Tui::new();
		let state = State::new(&player);

		let tick = Duration::from_millis(100);

		Application {
			player,
			tui,
			state,
			tick,
		}
	}

	pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) {
		let mut last = Instant::now();

		loop {
			terminal.draw(|f| self.tui.ui(f, &self.state)).unwrap();

			let timeout = self.tick.saturating_sub(last.elapsed());
			if event::poll(timeout).unwrap() {
				if let Event::Key(key) = event::read().unwrap() {
					match (key.code, key.modifiers) {
						(KeyCode::Char('q'), _) => return,
						(KeyCode::Char(' '), _) => self.player.toggle(),
						(KeyCode::Char('m'), _) => self.player.mute(),
						(KeyCode::Up, KeyModifiers::SHIFT) => self.player.i_vol(5f64),
						(KeyCode::Down, KeyModifiers::SHIFT) => self.player.d_vol(5f64),
						_ => {}
					}
				}
			}

			if last.elapsed() >= self.tick {
				self.state.tick(&self.player);
				last = Instant::now();
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

#[derive(Debug)]
struct Thing(u32);

fn main() {
	color_eyre::install().unwrap();

	let mut app = Application::new();
	app.player
		.queue("/home/may/tmp/music/album/Long Sought Rest - sacred objects/05 bleeding heart.mp3");

	app.start();
}
