use crate::state::State;
use conv::{ConvUtil, UnwrapOrSaturate};
use ratatui::{
	prelude::Rect,
	style::{Style, Stylize},
	text::Line,
	widgets::{Block, Borders, Clear, Padding, Paragraph},
	Frame,
};

mod utils;
mod window;

#[derive(Debug)]
pub enum Popup {
	Tags,
	List,
}

#[derive(Debug, Default)]
pub struct Tags {
	scroll: u16,
	scroll_max: u16,
	do_scroll: bool,
}

impl Tags {
	pub fn draw(&mut self, frame: &mut Frame, area: Rect, state: &State) {
		let block = Block::default()
			.title("tags")
			.borders(Borders::ALL)
			.padding(Padding::new(2, 2, 1, 1));
		let list = self.list(state);

		let par = if self.do_scroll {
			Paragraph::new(list).block(block).scroll((self.scroll, 0))
		} else {
			Paragraph::new(list).block(block)
		};

		frame.render_widget(Clear, area);
		frame.render_widget(par, area);
	}

	pub fn set_scroll(&mut self, area: Rect, state: &State) {
		let list = self.list(state);

		let lines = list.len().approx_as::<u16>().unwrap_or_saturate();
		let height = area.height.saturating_sub(4);

		let n_scroll_max = lines.saturating_sub(height);
		if n_scroll_max != self.scroll_max {
			self.scroll = 0;
			self.scroll_max = n_scroll_max;
		}

		if lines > height && !self.do_scroll {
			self.do_scroll = true;
		} else if lines <= height && self.do_scroll {
			self.scroll = 0;
			self.do_scroll = false;
		}
	}

	pub fn up(&mut self) {
		if self.do_scroll {
			self.scroll = self.scroll.saturating_sub(1);
		}
	}

	pub fn down(&mut self) {
		if self.do_scroll {
			self.scroll = u16::min(self.scroll_max, self.scroll + 1);
		}
	}

	fn list(&self, state: &State) -> Vec<Line> {
		let dimmed = Style::default().dim().italic();
		if let Some(track) = state.track.as_ref() {
			let underline = Style::default().underlined();

			let title = track
				.title()
				.map_or(Line::styled("none", dimmed), Line::from);
			let artist = track
				.artist()
				.map_or(Line::styled("none", dimmed), Line::from);
			let album = track
				.album()
				.map_or(Line::styled("none", dimmed), Line::from);
			let num = track.track().map_or(Line::styled("none", dimmed), |num| {
				Line::from(num.to_string())
			});

			vec![
				Line::styled("title", underline),
				title,
				Line::default(),
				Line::styled("artist", underline),
				artist,
				Line::default(),
				Line::styled("album", underline),
				album,
				Line::default(),
				Line::styled("track", underline),
				num,
			]
		} else {
			vec![Line::styled("no track playing", dimmed)]
		}
	}
}

#[derive(Debug, Default)]
pub struct Ui {
	tags: Tags,
	popup: Option<Popup>,
}

impl Ui {
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		let size = frame.size();
		let (window, seek) = window::layout(size);

		window::main(frame, window, state);
		window::seek(frame, seek, state);

		self.popup(frame, window, state);
	}

	fn popup(&mut self, frame: &mut Frame, main: Rect, state: &State) {
		let area = window::popup(main);
		match self.popup {
			Some(Popup::Tags) => {
				self.tags.set_scroll(area, state);
				self.tags.draw(frame, area, state);
			}
			Some(Popup::List) => todo!(),
			None => {}
		}
	}

	pub fn tags(&mut self) {
		match self.popup {
			Some(Popup::Tags) => self.popup = None,
			_ => self.popup = Some(Popup::Tags),
		}
	}

	pub fn up(&mut self) {
		match self.popup {
			Some(Popup::Tags) => self.tags.up(),
			Some(Popup::List) => todo!(),
			None => {}
		}
	}

	pub fn down(&mut self) {
		match self.popup {
			Some(Popup::Tags) => self.tags.down(),
			Some(Popup::List) => todo!(),
			None => {}
		}
	}

	pub fn list(&mut self) {
		match self.popup {
			Some(Popup::List) => self.popup = None,
			_ => self.popup = Some(Popup::List),
		}
	}

	pub fn esc(&mut self) {
		self.popup = None;
	}
}
