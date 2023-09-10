use crate::state::State;
use ratatui::{
	prelude::{Constraint, Direction, Layout, Rect},
	widgets::{Block, Borders},
	Frame,
};

#[derive(Debug, Default)]
pub struct Ui {}

impl Ui {
	#[allow(unused_variables)]
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		// todo!

		// println!("tick {:?}\r", state);

		let size = frame.size();
		let (window, seek) = self.layout(size);

		self.draw_main(frame, window);
		self.draw_seek(frame, seek);
	}

	fn draw_main(&self, frame: &mut Frame, area: Rect) {
		let block = Block::default().title("window").borders(Borders::ALL);
		frame.render_widget(block, area);
	}

	fn draw_seek(&self, frame: &mut Frame, area: Rect) {
		let block = Block::default().title("seek").borders(Borders::ALL);
		frame.render_widget(block, area);
	}

	fn layout(&self, size: Rect) -> (Rect, Rect) {
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(0), Constraint::Max(5)])
			.split(size);
		(chunks[0], chunks[1])
	}
}
