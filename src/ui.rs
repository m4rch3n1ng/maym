use crate::state::State;
use ratatui::{Frame, widgets::{Block, Borders, Clear}};

mod utils;
mod window;

#[derive(Debug)]
pub enum Popup {
	Tags,
}

#[derive(Debug, Default)]
pub struct Ui {
	popup: Option<Popup>,
}

impl Ui {
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		let size = frame.size();
		let (window, seek) = window::layout(size);

		window::main(frame, window, state);
		window::seek(frame, seek, state);

		if self.popup.is_some() {
			let block = Block::default().title("popup").borders(Borders::ALL);
			let area = window::popup(window);
			frame.render_widget(Clear, area);
			frame.render_widget(block, area);
		}
	}

	pub fn tags(&mut self) {
		match self.popup {
			Some(Popup::Tags) => self.popup = None,
			None => self.popup = Some(Popup::Tags),
		}
	}

	pub fn esc(&mut self) {
		self.popup = None;
	}
}
