use super::utils;
use crate::{
	config::{Child, Config, List},
	player::Player,
	queue::Queue,
	state::State,
};
use conv::{ConvUtil, UnwrapOrSaturate};
use ratatui::{
	prelude::Rect,
	style::{Modifier, Style, Stylize},
	text::Line,
	widgets::{Block, Clear, List as ListWidget, ListItem, ListState, Paragraph},
	Frame,
};

pub trait Popup {
	fn reset(&mut self) {
		self.set_pos(0);
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
		let height = utils::popup::block().inner(area).height;

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
		let block = utils::popup::block().title(" tags ");
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
		let block = utils::popup::block().title(" lyrics ");
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

#[derive(Debug)]
pub struct Tracks {
	state: ListState,
	len: usize,
	page: Option<usize>,
}

impl Tracks {
	pub fn new(queue: &Queue) -> Self {
		let idx = queue.idx().unwrap_or(0);
		let state = ListState::default()
			.with_selected(Some(idx))
			.with_offset(usize::MAX);

		Tracks {
			state,
			len: queue.tracks().len(),
			page: None,
		}
	}

	pub fn draw(&mut self, frame: &mut Frame, area: Rect, queue: &Queue) {
		let items = tracks_list(queue);

		let block = utils::popup::block().title(" tracks ");
		let inner = block.inner(area);
		let (title_area, list_area) = utils::popup::double_layout(inner);

		frame.render_widget(Clear, area);
		frame.render_widget(block, area);

		let page = usize::from(list_area.height);
		if self.page.is_none() {
			*self.state.offset_mut() = self.len.saturating_sub(page);
		}
		self.page = Some(page);

		let path = queue.path();
		let line = path.map_or(
			Line::styled("nothing playing", Style::default().bold().dim().italic()),
			|path| Line::styled(format!(">> {:?}", path), Style::default().bold()),
		);
		let title = Paragraph::new(line).block(Block::default());
		frame.render_widget(title, title_area);

		let list = ListWidget::new(items)
			.block(Block::default())
			.style(Style::default().dim())
			.highlight_style(Style::default().remove_modifier(Modifier::DIM));

		frame.render_stateful_widget(list, list_area, &mut self.state);
	}

	fn offset(&self) -> usize {
		self.page
			.map_or(usize::MAX, |page| self.len.saturating_sub(page))
	}

	pub fn reset(&mut self, queue: &Queue) {
		self.state.select(Some(0));
		self.len = queue.tracks().len();
	}

	pub fn select(&mut self, idx: usize) {
		self.state.select(Some(idx));

		let offset = self.offset();
		*self.state.offset_mut() = offset;
	}

	// todo wrap around ?
	pub fn down(&mut self) {
		let idx = self
			.state
			.selected()
			.map(|i| usize::min(self.len.saturating_sub(1), i.saturating_add(1)));
		self.state.select(idx);
	}

	// todo wrap around ?
	pub fn up(&mut self) {
		let idx = self.state.selected().map(|i| i.saturating_sub(1));
		self.state.select(idx);
	}

	pub fn enter(&self, player: &mut Player, queue: &mut Queue) {
		let idx = self.state.selected().unwrap();
		queue.select_idx(idx, player).unwrap();
	}
}

// todo associated fn perhaps
fn tracks_list(queue: &Queue) -> Vec<ListItem> {
	queue
		.tracks()
		.iter()
		.map(|track| track.line(queue))
		.map(Line::from)
		.map(ListItem::new)
		.collect()
}

#[derive(Debug)]
pub struct Lists {
	state: ListState,
	list: List,
}

impl Lists {
	pub fn new(config: &Config) -> Self {
		let list = config.lists()[0].clone();
		let state = ListState::default().with_selected(Some(0));

		Lists { state, list }
	}

	pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
		let children = self.list.children();
		let items = lists_list(&children);

		let block = utils::popup::block().title(" lists ");
		let list = ListWidget::new(items)
			.block(block)
			.style(Style::default().dim())
			.highlight_style(Style::default().remove_modifier(Modifier::DIM));

		frame.render_stateful_widget(list, area, &mut self.state);
	}

	// todo wrap around
	pub fn down(&mut self) {
		let len = self.list.children().len();
		let next = self.state.selected().map(|i| usize::min(len, i + 1));
		self.state.select(next);
	}

	// todo wrap around
	pub fn up(&mut self) {
		let prev = self.state.selected().map(|i| i.saturating_sub(1));
		self.state.select(prev);
	}

	fn curr(&self) -> Child {
		let children = self.list.children();
		let idx = self.state.selected().unwrap();
		children[idx].clone()
	}

	pub fn right(&mut self) {
		let child = self.curr();
		if let Some(list) = child.list() {
			self.list = list.clone();
			self.state.select(Some(0));
		}
	}

	pub fn left(&mut self) {
		let curr = &self.list;

		if let Some(list) = self.list.parent() {
			let idx = list.children().iter().position(|child| child == curr);
			let idx = idx.unwrap_or(0);

			self.list = list;
			self.state.select(Some(idx));
		}
	}

	pub fn enter(&mut self, player: &mut Player, queue: &mut Queue) {
		let child = self.curr();
		match child {
			Child::List(list) => {
				self.list = list.clone();
				self.state.select(Some(0));
			}
			Child::Mp3(path) => {
				queue.queue(&self.list.path).unwrap();
				queue.select_path(&path, player);
			}
		}
	}
}

fn lists_list(children: &[Child]) -> Vec<ListItem> {
	children
		.iter()
		.map(|child| child.line())
		.map(ListItem::new)
		.collect()
}
