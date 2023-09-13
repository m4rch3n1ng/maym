use self::popup::{Lyrics, Popup, Tags};
use crate::state::State;
use ratatui::{prelude::Rect, Frame};

mod popup;
mod utils;
mod window;

#[derive(Debug, Clone, Copy)]
pub enum Popups {
	List,
	Tags,
	Lyrics,
}

#[derive(Debug, Default)]
pub struct Ui {
	tags: Tags,
	lyrics: Lyrics,
	popup: Option<Popups>,
}

impl Ui {
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		let size = frame.size();
		let (window, seek) = window::layout(size);

		window::main(frame, window, state);
		window::seek(frame, seek, state);

		self.popup(frame, window, state);
	}

	// todo make generic maybe ?
	fn popup(&mut self, frame: &mut Frame, main: Rect, state: &State) {
		let area = window::popup(main);
		match self.popup {
			Some(Popups::Tags) => {
				self.tags.update_scroll(area, state);
				self.tags.draw(frame, area, state);
			}
			Some(Popups::List) => todo!(),
			Some(Popups::Lyrics) => {
				self.lyrics.update_scroll(area, state);
				self.lyrics.draw(frame, area, state);
			}
			None => {}
		}
	}

	pub fn list(&mut self) {
		match self.popup {
			Some(Popups::List) => self.popup = None,
			_ => self.popup = Some(Popups::List),
		}
	}

	pub fn tags(&mut self) {
		if let Some(Popups::Tags) = self.popup {
			self.popup = None;
		} else {
			self.tags.init();
			self.popup = Some(Popups::Tags);
		}
	}

	pub fn lyrics(&mut self) {
		if let Some(Popups::Lyrics) = self.popup {
			self.popup = None;
		} else {
			self.lyrics.init();
			self.popup = Some(Popups::Lyrics);
		}
	}

	pub fn up(&mut self) {
		match self.popup {
			Some(Popups::Tags) => self.tags.up(),
			Some(Popups::List) => todo!(),
			Some(Popups::Lyrics) => self.lyrics.up(),
			None => {}
		}
	}

	pub fn down(&mut self) {
		match self.popup {
			Some(Popups::Tags) => self.tags.down(),
			Some(Popups::List) => todo!(),
			Some(Popups::Lyrics) => self.lyrics.down(),
			None => {}
		}
	}

	pub fn esc(&mut self) {
		self.popup = None;
	}
}
