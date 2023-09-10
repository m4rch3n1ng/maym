use std::time::Duration;

use crate::state::State;
use ratatui::{
	prelude::{Constraint, Direction, Layout, Rect},
	text::Text,
	widgets::{Block, Borders, Padding, Paragraph},
	Frame,
};

#[derive(Debug, Default)]
pub struct Ui {}

impl Ui {
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		let size = frame.size();
		let (window, seek) = self.layout(size);

		self.draw_main(frame, window);
		self.draw_seek(frame, seek, state);
	}

	fn draw_main(&self, frame: &mut Frame, area: Rect) {
		let block = Block::default().title("window").borders(Borders::ALL);
		frame.render_widget(block, area);
	}

	fn draw_seek(&self, frame: &mut Frame, area: Rect, state: &State) {
		if let Some(elapsed) = state.elapsed() {
			let elapsed = fmt_duration(elapsed);
			let duration = fmt_duration(state.duration.unwrap());
			let thing = format!("{} / {}", elapsed, duration);

			let text = Text::from(thing);
			let block = Block::default()
				.title("seek")
				.borders(Borders::ALL)
				.padding(Padding::new(2, 0, 1, 1));
			let par = Paragraph::new(text).block(block);
			frame.render_widget(par, area);
		} else {
			let block = Block::default().title("seek").borders(Borders::ALL);
			frame.render_widget(block, area);
		}
	}

	fn layout(&self, size: Rect) -> (Rect, Rect) {
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(0), Constraint::Max(5)])
			.split(size);
		(chunks[0], chunks[1])
	}
}

fn fmt_duration(duration: Duration) -> String {
	let min = (duration.as_secs() / 60) % 60;
	let sec = duration.as_secs() % 60;

	format!("{:0>2}:{:0>2}", min, sec)
}
