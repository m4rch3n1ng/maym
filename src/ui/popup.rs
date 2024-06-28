use super::utils;
use crate::{
	config::{Child, Config, List},
	player::Player,
	queue::{Queue, QueueError, Track},
	state::State,
};
use ratatui::{
	layout::Rect,
	style::{Modifier, Style, Stylize},
	terminal::Frame,
	text::Line,
	widgets::{Block, Clear, List as ListWidget, ListItem, ListState, Paragraph},
};

#[derive(Debug)]
pub struct Popup<T: PopupTrait> {
	inner: T,
	pos: u16,
	scroll_amt: u16,
	do_scroll: bool,
}

impl<T: PopupTrait + Default> Default for Popup<T> {
	fn default() -> Self {
		Popup {
			inner: T::default(),
			pos: 0,
			scroll_amt: 0,
			do_scroll: false,
		}
	}
}

impl<T: PopupTrait> Popup<T> {
	pub fn draw(&mut self, frame: &mut Frame, area: Rect, state: &State) {
		self.update_scroll(area, state);

		let title = self.inner.title();
		let block = utils::popup::block().title(title);
		let list = self.inner.list(state);

		let par = if self.do_scroll {
			Paragraph::new(list).block(block).scroll((self.pos, 0))
		} else {
			Paragraph::new(list).block(block)
		};

		frame.render_widget(Clear, area);
		frame.render_widget(par, area);
	}

	fn update_scroll(&mut self, area: Rect, state: &State) {
		let list = self.inner.list(state);

		let lines = usize::min(list.len(), u16::MAX as usize) as u16;
		let height = utils::popup::block().inner(area).height;

		let n_scroll_max = lines.saturating_sub(height);
		if n_scroll_max != self.scroll_amt {
			self.pos = 0;
			self.scroll_amt = n_scroll_max;
		}

		if lines > height && !self.do_scroll {
			self.do_scroll = true;
		} else if lines <= height && self.do_scroll {
			self.pos = 0;
			self.do_scroll = false;
		}
	}

	pub fn reset(&mut self) {
		self.pos = 0;
	}

	pub fn up(&mut self) {
		if self.do_scroll {
			let pos = self.pos.saturating_sub(1);
			self.pos = pos;
		}
	}

	pub fn down(&mut self) {
		if self.do_scroll {
			let pos = u16::min(self.scroll_amt, self.pos + 1);
			self.pos = pos;
		}
	}
}

pub trait PopupTrait {
	fn list<'s>(&self, state: &'s State) -> Vec<Line<'s>>;

	fn title(&self) -> &'static str;
}

#[derive(Debug, Default)]
pub struct Lyrics;

impl PopupTrait for Lyrics {
	fn list<'s>(&self, state: &'s State) -> Vec<Line<'s>> {
		let dimmed = Style::default().dim().italic();

		if let Some(track) = state.track.as_ref() {
			if let Some(lyrics) = track.lyrics() {
				lyrics.lines().map(Line::from).collect()
			} else {
				vec![utils::widgets::line("track has no lyrics", dimmed)]
			}
		} else {
			vec![utils::widgets::line("no track playing", dimmed)]
		}
	}

	fn title(&self) -> &'static str {
		" lyrics "
	}
}

#[derive(Debug, Default)]
pub struct Tags;

impl PopupTrait for Tags {
	fn list<'s>(&self, state: &'s State) -> Vec<Line<'s>> {
		let dimmed = Style::default().dim().italic();
		if let Some(track) = state.track.as_ref() {
			let underline = Style::default().underlined();

			let title = track
				.title()
				.map_or(utils::widgets::line("none", dimmed), Line::from);
			let artist = track
				.artist()
				.map_or(utils::widgets::line("none", dimmed), Line::from);
			let album = track
				.album()
				.map_or(utils::widgets::line("none", dimmed), Line::from);
			let num = track
				.track()
				.map_or(utils::widgets::line("none", dimmed), |num| {
					Line::from(num.to_string())
				});
			let path = Line::from(track.path.as_str());

			vec![
				utils::widgets::line("title", underline),
				title,
				Line::default(),
				utils::widgets::line("artist", underline),
				artist,
				Line::default(),
				utils::widgets::line("album", underline),
				album,
				Line::default(),
				utils::widgets::line("track", underline),
				num,
				Line::default(),
				utils::widgets::line("path", underline),
				path,
			]
		} else {
			vec![utils::widgets::line("no track playing", dimmed)]
		}
	}

	fn title(&self) -> &'static str {
		" tags "
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
			utils::widgets::line("nothing playing", Style::default().bold().dim().italic()),
			|path| utils::widgets::line(format!(">> {:?}", path), Style::default().bold()),
		);
		let title = Paragraph::new(line).block(Block::default());
		frame.render_widget(title, title_area);

		let items = self.items(queue);
		let list = ListWidget::new(items)
			.block(Block::default())
			.style(Style::default().dim())
			.highlight_style(Style::default().remove_modifier(Modifier::DIM));

		frame.render_stateful_widget(list, list_area, &mut self.state);
	}

	fn items<'q>(&self, queue: &'q Queue) -> Vec<ListItem<'q>> {
		queue
			.tracks()
			.iter()
			.map(|track| track.line(queue))
			.map(Line::from)
			.map(ListItem::new)
			.collect()
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

	pub fn down(&mut self) {
		let max = self.len.saturating_sub(1);
		let idx = self
			.state
			.selected()
			.map(|i| if i == max { 0 } else { i.saturating_add(1) });
		self.state.select(idx);
	}

	pub fn up(&mut self) {
		let idx = self.state.selected().map(|i| {
			if i == 0 {
				self.len.saturating_sub(1)
			} else {
				i.saturating_sub(1)
			}
		});
		self.state.select(idx);
	}

	pub fn page_down(&mut self) {
		if let Some(page) = self.page {
			let idx = self
				.state
				.selected()
				.map(|i| usize::min(self.len.saturating_sub(1), i.saturating_add(page)));
			self.state.select(idx);
			*self.state.offset_mut() = usize::min(
				self.len.saturating_sub(page),
				self.state.offset().saturating_add(page),
			);
		}
	}

	pub fn page_up(&mut self) {
		if let Some(page) = self.page {
			let idx = self.state.selected().map(|i| i.saturating_sub(page));
			self.state.select(idx);
			*self.state.offset_mut() = self.state.offset().saturating_sub(page);
		}
	}

	pub fn home(&mut self) {
		self.state.select(Some(0));
		*self.state.offset_mut() = 0;
	}

	pub fn end(&mut self) {
		let len = self.len.saturating_sub(1);
		self.state.select(Some(len));
		*self.state.offset_mut() = self.offset();
	}

	pub fn enter(&self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let idx = self.state.selected().expect("state should always be Some");
		queue.select_idx(idx, player)?;
		Ok(())
	}
}

#[derive(Debug)]
enum ListType<'a> {
	Child(Child, &'a List),
	List(&'a List),
}

#[derive(Debug)]
pub struct Lists {
	state: ListState,
	lists: Vec<List>,
	list: Option<List>,
	page: Option<usize>,
}

impl Lists {
	pub fn new(config: &Config, queue: &Queue) -> Self {
		let lists = config.lists().to_owned();

		let list = if let Some(path) = queue.path() {
			lists.iter().find_map(|list| list.find(path))
		} else {
			None
		};

		let idx = if let Some(track) = queue.track() {
			if let Some(ref list) = list {
				list.children()
					.iter()
					.enumerate()
					.find_map(|(i, child)| (child == track).then_some(i))
			} else {
				None
			}
		} else {
			None
		};
		let idx = idx.unwrap_or(0);
		let state = ListState::default().with_selected(Some(idx));

		Lists {
			state,
			lists,
			list,
			page: None,
		}
	}

	pub fn draw(&mut self, frame: &mut Frame, area: Rect, queue: &Queue) {
		let children = self.list.as_ref().map(|list| list.children());
		let items = if let Some(ref children) = children {
			lists_list(children, queue)
		} else {
			root_list(&self.lists, queue)
		};

		let block = utils::popup::block().title(" lists ");
		let inner = block.inner(area);
		let (title_area, list_area) = utils::popup::double_layout(inner);

		frame.render_widget(Clear, area);
		frame.render_widget(block, area);

		let page = usize::from(list_area.height);
		if self.page.is_none() {
			*self.state.offset_mut() = self.len().saturating_sub(page);
		}
		self.page = Some(page);

		let line = self.list.as_ref().map_or(
			utils::widgets::line("<< \"/\"", Style::default().bold()),
			|list| utils::widgets::line(format!("<< {:?}", list.path), Style::default().bold()),
		);
		let paragraph = Paragraph::new(line);
		frame.render_widget(paragraph, title_area);

		let list = ListWidget::new(items)
			.block(Block::default())
			.style(Style::default().dim())
			.highlight_style(Style::default().remove_modifier(Modifier::DIM));

		frame.render_stateful_widget(list, list_area, &mut self.state);
	}

	fn len(&self) -> usize {
		if let Some(ref list) = self.list {
			list.children().len()
		} else {
			self.lists.len()
		}
	}

	fn offset(&self) -> usize {
		self.page
			.map_or(usize::MAX, |page| self.len().saturating_sub(page))
	}

	pub fn select(&mut self, track: &Track) {
		if let Some(ref list) = self.list {
			let children = list.children();
			let idx = children.iter().position(|child| child == track);
			let idx = idx.unwrap_or(0);

			self.state.select(Some(idx));
			*self.state.offset_mut() = self.offset();
		}
	}

	pub fn down(&mut self) {
		let max = self.len().saturating_sub(1);
		let idx = self
			.state
			.selected()
			.map(|i| if i == max { 0 } else { i.saturating_add(1) });

		self.state.select(idx);
	}

	pub fn up(&mut self) {
		let idx = self.state.selected().map(|i| {
			if i == 0 {
				self.len().saturating_sub(1)
			} else {
				i.saturating_sub(1)
			}
		});

		self.state.select(idx);
	}

	pub fn page_down(&mut self) {
		if let Some(page) = self.page {
			let idx = self
				.state
				.selected()
				.map(|i| usize::min(self.len().saturating_sub(1), i.saturating_add(page)));
			self.state.select(idx);
			*self.state.offset_mut() = usize::min(
				self.len().saturating_sub(page),
				self.state.offset().saturating_add(page),
			);
		}
	}

	pub fn page_up(&mut self) {
		if let Some(page) = self.page {
			let idx = self.state.selected().map(|i| i.saturating_sub(page));
			self.state.select(idx);
			*self.state.offset_mut() = self.state.offset().saturating_sub(page);
		}
	}

	pub fn home(&mut self) {
		self.state.select(Some(0));
		*self.state.offset_mut() = 0;
	}

	pub fn end(&mut self) {
		let len = self.len().saturating_sub(1);
		self.state.select(Some(len));
		*self.state.offset_mut() = self.offset();
	}

	fn curr(&self) -> ListType {
		if let Some(ref list) = self.list {
			let children = list.children();
			let idx = self.state.selected().expect("state should always be Some");

			let child = children[idx].clone();
			ListType::Child(child, list)
		} else {
			let idx = self.state.selected().expect("state should always be Some");
			let list = &self.lists[idx];
			ListType::List(list)
		}
	}

	/// overwrites `self.list` and sets the index for `self.state`
	fn set(&mut self, list: Option<List>, idx: usize) {
		self.list = list;
		self.state.select(Some(idx));
		*self.state.offset_mut() = self.offset();
	}

	pub fn right(&mut self) {
		let curr = self.curr();

		match curr {
			ListType::Child(child, _) => {
				if let Some(list) = child.list() {
					let list = list.clone();
					self.set(Some(list), 0);
				}
			}
			ListType::List(list) => {
				let list = list.clone();
				self.set(Some(list), 0);
			}
		}
	}

	pub fn left(&mut self) {
		if let Some(ref mut list) = self.list {
			// warn: list is now invalid as the parent is now `None`
			if let Some(parent) = list.parent() {
				let idx = parent.children().iter().position(|child| child == list);
				let idx = idx.unwrap_or(0);

				self.set(Some(parent), idx);
			} else {
				let idx = self.lists.iter().position(|root| root == list);
				let idx = idx.unwrap_or(0);

				self.set(None, idx);
			}
		}
	}

	pub fn enter(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let curr = self.curr();

		match curr {
			ListType::List(list) => {
				let list = list.clone();
				self.set(Some(list), 0);
			}
			ListType::Child(child, parent) => match child {
				Child::List(list) => {
					let list = list.clone();
					self.set(Some(list), 0);
				}
				Child::Mp3(path) => {
					queue.queue(&parent.path)?;
					queue.select_path(&path, player)?;
				}
			},
		}

		Ok(())
	}

	pub fn space(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let curr = self.curr();

		match curr {
			ListType::List(list) => {
				queue.queue(&list.path)?;
				let _ = queue.next(player);
			}
			ListType::Child(child, parent) => match child {
				Child::List(list) => {
					queue.queue(&list.path)?;
					let _ = queue.next(player);
				}
				Child::Mp3(track) => {
					queue.queue(&parent.path)?;
					queue.select_path(&track, player)?;
				}
			},
		}

		Ok(())
	}
}

fn lists_list<'a>(children: &'a [Child], queue: &Queue) -> Vec<ListItem<'a>> {
	children
		.iter()
		.map(|child| child.line(queue))
		.map(ListItem::new)
		.collect()
}

fn root_list<'a>(lists: &'a [List], queue: &Queue) -> Vec<ListItem<'a>> {
	lists
		.iter()
		.map(|root| root.line(queue))
		.map(ListItem::new)
		.collect()
}
