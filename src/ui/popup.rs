use crate::state::State;
use conv::{ConvUtil, UnwrapOrSaturate};
use ratatui::{
	prelude::Rect,
	style::{Style, Stylize},
	text::Line,
	widgets::{Block, Borders, Clear, Padding, Paragraph},
	Frame,
};

pub trait Popup {
	fn init(&mut self) {
		self.set_pos(0);
	}

	fn block(&self) -> Block {
		Block::default()
			.borders(Borders::ALL)
			.border_style(Style::default().dim())
			.padding(Padding::new(2, 2, 1, 1))
	}

	fn pos(&self) -> u16;

	fn set_pos(&mut self, amt: u16);

	fn set_scroll_amt(&mut self, amt: u16);

	fn scroll_amt(&self) -> u16;

	fn do_scroll(&self) -> bool;

	fn set_do_scroll(&mut self, val: bool);

	fn update_scroll(&mut self, area: Rect, state: &State) {
		let list = self.list(state);

		let lines = list.len().approx_as::<u16>().unwrap_or_saturate();
		let height = self.block().inner(area).height;

		let n_scroll_max = lines.saturating_sub(height);
		if n_scroll_max != self.scroll_amt() {
			self.set_pos(0);
			self.set_scroll_amt(n_scroll_max);
		}

		if lines > height && !self.do_scroll() {
			self.set_do_scroll(true);
		} else if lines <= height && self.do_scroll() {
			self.set_pos(0);
			self.set_do_scroll(false);
		}
	}

	fn list(&self, state: &State) -> Vec<Line>;

	fn up(&mut self) {
		if self.do_scroll() {
			let pos = self.pos().saturating_sub(1);
			self.set_pos(pos);
		}
	}

	fn down(&mut self) {
		if self.do_scroll() {
			let pos = u16::min(self.scroll_amt(), self.pos() + 1);
			self.set_pos(pos);
		}
	}
}

#[derive(Debug, Default)]
pub struct Tags {
	pos: u16,
	scroll_amt: u16,
	do_scroll: bool,
}

impl Tags {
	pub fn draw(&self, frame: &mut Frame, area: Rect, state: &State) {
		let block = self.block().title("tags");
		let list = self.list(state);

		let par = if self.do_scroll {
			Paragraph::new(list).block(block).scroll((self.pos, 0))
		} else {
			Paragraph::new(list).block(block)
		};

		frame.render_widget(Clear, area);
		frame.render_widget(par, area);
	}
}

impl Popup for Tags {
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

	fn do_scroll(&self) -> bool {
		self.do_scroll
	}

	fn pos(&self) -> u16 {
		self.pos
	}

	fn scroll_amt(&self) -> u16 {
		self.scroll_amt
	}

	fn set_do_scroll(&mut self, val: bool) {
		self.do_scroll = val;
	}

	fn set_pos(&mut self, amt: u16) {
		self.pos = amt;
	}

	fn set_scroll_amt(&mut self, amt: u16) {
		self.scroll_amt = amt;
	}
}

#[derive(Debug, Default)]
pub struct Lyrics {
	pos: u16,
	do_scroll: bool,
	scroll_amt: u16,
}

impl Lyrics {
	pub fn draw(&self, frame: &mut Frame, area: Rect, state: &State) {
		let block = self.block().title("lyrics");
		let list = self.list(state);

		// wrap depends on https://github.com/ratatui-org/ratatui/issues/136
		let par = if self.do_scroll {
			Paragraph::new(list).block(block).scroll((self.pos, 0))
		} else {
			Paragraph::new(list).block(block)
		};

		frame.render_widget(Clear, area);
		frame.render_widget(par, area);
	}
}

impl Popup for Lyrics {
	// todo perf
	// talking about rust performance when it comes to cloning strings
	// is incredibly funny when coming from a js background btw
	fn list(&self, state: &State) -> Vec<Line> {
		let dimmed = Style::default().dim().italic();

		if let Some(track) = state.track.as_ref() {
			if let Some(lyrics) = track.lyrics().as_ref() {
				lyrics
					.lines()
					.map(ToOwned::to_owned)
					.map(Line::from)
					.collect()
			} else {
				vec![Line::styled("track has no lyrics", dimmed)]
			}
		} else {
			vec![Line::styled("no track playing", dimmed)]
		}
	}

	fn do_scroll(&self) -> bool {
		self.do_scroll
	}

	fn pos(&self) -> u16 {
		self.pos
	}

	fn scroll_amt(&self) -> u16 {
		self.scroll_amt
	}

	fn set_do_scroll(&mut self, val: bool) {
		self.do_scroll = val;
	}

	fn set_pos(&mut self, amt: u16) {
		self.pos = amt;
	}

	fn set_scroll_amt(&mut self, amt: u16) {
		self.scroll_amt = amt;
	}
}
