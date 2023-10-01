use self::popup::{Lists, Lyrics, Popup, Tags, Tracks};
use crate::{
	config::Config,
	player::Player,
	queue::{Queue, QueueError},
	state::State,
};
use ratatui::{prelude::Rect, Frame};

mod popup;
pub mod utils;
mod window;

#[derive(Debug, Clone, Copy)]
pub enum Popups {
	Tracks,
	Lists,
	Tags,
	Lyrics,
}

#[derive(Debug)]
pub struct Ui {
	tags: Tags,
	lyrics: Lyrics,
	tracks: Tracks,
	lists: Lists,
	pub popup: Option<Popups>,
}

impl Ui {
	pub fn new(queue: &Queue, config: &Config) -> Self {
		Ui {
			tags: Tags::default(),
			lyrics: Lyrics::default(),
			tracks: Tracks::new(queue),
			lists: Lists::new(config, queue),
			popup: None,
		}
	}

	pub fn draw(&mut self, frame: &mut Frame, state: &State, queue: &Queue) {
		let size = frame.size();
		let (window, seek) = window::layout(size);

		window::main(frame, window, state);
		window::seek(frame, seek, state);

		self.popup(frame, window, state, queue);
	}

	// todo make generic maybe ?
	fn popup(&mut self, frame: &mut Frame, main: Rect, state: &State, queue: &Queue) {
		let area = window::popup(main);
		match self.popup {
			Some(Popups::Tags) => {
				self.tags.update_scroll(area, state);
				self.tags.draw(frame, area, state);
			}
			Some(Popups::Tracks) => self.tracks.draw(frame, area, queue),
			Some(Popups::Lyrics) => {
				self.lyrics.update_scroll(area, state);
				self.lyrics.draw(frame, area, state);
			}
			Some(Popups::Lists) => self.lists.draw(frame, area, queue),
			None => {}
		}
	}

	pub fn is_popup(&self) -> bool {
		self.popup.is_some()
	}

	pub fn reset(&mut self, queue: &Queue) {
		self.tags.reset();
		self.lyrics.reset();

		if !matches!(self.popup, Some(Popups::Tracks)) {
			if let Some(idx) = queue.idx() {
				self.tracks.select(idx);
			}
		}

		if !matches!(self.popup, Some(Popups::Lists)) {
			if let Some(track) = queue.track() {
				self.lists.select(track);
			}
		}
	}

	pub fn reset_q(&mut self, queue: &Queue) {
		self.tracks.reset(queue);
	}

	pub fn lists(&mut self) {
		if let Some(Popups::Lists) = self.popup {
			self.popup = None;
		} else {
			self.popup = Some(Popups::Lists);
		}
	}

	pub fn tracks(&mut self) {
		if let Some(Popups::Tracks) = self.popup {
			self.popup = None;
		} else {
			self.popup = Some(Popups::Tracks);
		}
	}

	pub fn tags(&mut self) {
		if let Some(Popups::Tags) = self.popup {
			self.popup = None;
		} else {
			self.popup = Some(Popups::Tags);
		}
	}

	pub fn lyrics(&mut self) {
		if let Some(Popups::Lyrics) = self.popup {
			self.popup = None;
		} else {
			self.popup = Some(Popups::Lyrics);
		}
	}

	pub fn up(&mut self) {
		match self.popup {
			Some(Popups::Tags) => self.tags.up(),
			Some(Popups::Tracks) => self.tracks.up(),
			Some(Popups::Lyrics) => self.lyrics.up(),
			Some(Popups::Lists) => self.lists.up(),
			None => {}
		}
	}

	pub fn down(&mut self) {
		match self.popup {
			Some(Popups::Tags) => self.tags.down(),
			Some(Popups::Tracks) => self.tracks.down(),
			Some(Popups::Lyrics) => self.lyrics.down(),
			Some(Popups::Lists) => self.lists.down(),
			None => {}
		}
	}

	pub fn pg_up(&mut self) {
		match self.popup {
			Some(Popups::Tracks) => self.tracks.page_up(),
			Some(Popups::Lists) => self.lists.page_up(),
			_ => {}
		}
	}

	pub fn pg_down(&mut self) {
		match self.popup {
			Some(Popups::Tracks) => self.tracks.page_down(),
			Some(Popups::Lists) => self.lists.page_down(),
			_ => {}
		}
	}

	pub fn home(&mut self) {
		match self.popup {
			Some(Popups::Tracks) => self.tracks.home(),
			Some(Popups::Lists) => self.lists.home(),
			_ => {}
		}
	}

	pub fn end(&mut self) {
		match self.popup {
			Some(Popups::Tracks) => self.tracks.end(),
			Some(Popups::Lists) => self.lists.end(),
			_ => {}
		}
	}

	pub fn right(&mut self) {
		if let Some(Popups::Lists) = self.popup {
			self.lists.right();
		}
	}

	pub fn left(&mut self) {
		if let Some(Popups::Lists) = self.popup {
			self.lists.left();
		}
	}

	pub fn backspace(&mut self) {
		if let Some(Popups::Lists) = self.popup {
			self.lists.left();
		}
	}

	pub fn enter(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		match self.popup {
			Some(Popups::Tracks) => self.tracks.enter(player, queue),
			Some(Popups::Lists) => self.lists.enter(player, queue),
			_ => Ok(()),
		}
	}

	pub fn space(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		match self.popup {
			Some(Popups::Tracks) => self.tracks.enter(player, queue),
			Some(Popups::Lists) => self.lists.space(player, queue),
			_ => Ok(()),
		}
	}

	pub fn esc(&mut self) {
		self.popup = None;
	}
}
