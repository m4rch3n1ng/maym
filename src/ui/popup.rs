use super::utils;
use crate::{
	config::{Child, Config, List},
	player::Player,
	queue::{Queue, QueueError},
	ui::Popup,
};
use ratatui::{
	Frame,
	layout::Rect,
	style::{Modifier, Style, Stylize},
	text::Line,
	widgets::{Block, Clear, List as ListWidget, ListItem, ListState, Paragraph},
};

#[derive(Debug)]
pub struct TextPopup {
	inner: fn(&Queue) -> Vec<Line<'_>>,
	title: &'static str,
	scroll: u16,
	max_scroll: u16,
}

impl TextPopup {
	fn new(title: &'static str, inner: fn(&Queue) -> Vec<Line<'_>>) -> TextPopup {
		TextPopup {
			inner,
			title,
			scroll: 0,
			max_scroll: 0,
		}
	}

	fn update_scroll(&mut self, area: Rect, list: &[Line<'_>]) {
		let lines = usize::min(list.len(), u16::MAX as usize) as u16;
		let height = utils::popup::block().inner(area).height;

		self.max_scroll = lines.saturating_sub(height);
		self.scroll = self.scroll.clamp(0, self.max_scroll);
	}
}

impl Popup for TextPopup {
	fn draw(&mut self, frame: &mut Frame, area: Rect, queue: &Queue) {
		let block = utils::popup::block().title(self.title);
		let list = (self.inner)(queue);

		self.update_scroll(area, &list);

		let par = Paragraph::new(list).block(block).scroll((self.scroll, 0));

		frame.render_widget(Clear, area);
		frame.render_widget(par, area);
	}

	fn change_track(&mut self, _queue: &Queue) {
		self.scroll = 0;
	}

	fn up(&mut self) {
		self.scroll = self.scroll.saturating_sub(1);
	}

	fn down(&mut self) {
		self.scroll = self.scroll.saturating_add(1).min(self.max_scroll);
	}

	fn home(&mut self) {
		self.scroll = 0;
	}

	fn end(&mut self) {
		self.scroll = self.max_scroll;
	}
}

pub fn lyrics() -> TextPopup {
	TextPopup::new(" lyrics ", |state| {
		let dimmed = Style::default().dim().italic();

		if let Some(track) = state.track() {
			if let Some(lyrics) = track.lyrics() {
				lyrics.lines().map(Line::from).collect()
			} else {
				vec![utils::widgets::line("track has no lyrics", dimmed)]
			}
		} else {
			vec![utils::widgets::line("no track playing", dimmed)]
		}
	})
}

pub fn tags() -> TextPopup {
	TextPopup::new(" tags ", |state| {
		let dimmed = Style::default().dim().italic();
		if let Some(track) = state.track() {
			let underline = Style::default().underlined();

			let title = track
				.title()
				.map_or_else(|| utils::widgets::line("none", dimmed), Line::from);
			let artist = track
				.artist()
				.map_or_else(|| utils::widgets::line("none", dimmed), Line::from);
			let album = track
				.album()
				.map_or_else(|| utils::widgets::line("none", dimmed), Line::from);
			let num = track.track().map_or_else(
				|| utils::widgets::line("none", dimmed),
				|num| Line::from(num.to_string()),
			);
			let path = Line::from(track.path().as_str());

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
	})
}

#[derive(Debug)]
pub struct Tracks {
	state: ListState,
	len: usize,
	page: Option<usize>,
}

impl Tracks {
	pub fn new(queue: &Queue) -> Self {
		let idx = queue.index().unwrap_or(0);
		let state = ListState::default()
			.with_selected(Some(idx))
			.with_offset(usize::MAX);

		Tracks {
			state,
			len: queue.tracks().len(),
			page: None,
		}
	}
}

impl Tracks {
	fn items(queue: &Queue) -> Vec<ListItem<'_>> {
		queue
			.tracks()
			.iter()
			.map(|track| track.line(queue))
			.map(ListItem::new)
			.collect()
	}

	fn offset(&self) -> usize {
		self.page
			.map_or(usize::MAX, |page| self.len.saturating_sub(page))
	}
}

impl Popup for Tracks {
	fn draw(&mut self, frame: &mut Frame, area: Rect, queue: &Queue) {
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
		let line = path.map_or_else(
			|| utils::widgets::line("nothing playing", Style::default().bold().dim().italic()),
			|path| utils::widgets::line(format!(">> {path:?}"), Style::default().bold()),
		);
		let title = Paragraph::new(line).block(Block::default());
		frame.render_widget(title, title_area);

		let items = Tracks::items(queue);
		let list = ListWidget::new(items)
			.block(Block::default())
			.style(Style::default().dim())
			.highlight_style(Style::default().remove_modifier(Modifier::DIM));

		frame.render_stateful_widget(list, list_area, &mut self.state);
	}

	fn change_track(&mut self, queue: &Queue) {
		let Some(index) = queue.index() else { return };
		self.state.select(Some(index));

		let offset = self.offset();
		*self.state.offset_mut() = offset;
	}

	fn change_queue(&mut self, queue: &Queue) {
		self.state.select(Some(0));
		self.len = queue.tracks().len();
	}

	fn down(&mut self) {
		let max = self.len.saturating_sub(1);
		let idx = self
			.state
			.selected()
			.map(|i| if i == max { 0 } else { i.saturating_add(1) });
		self.state.select(idx);
	}

	fn up(&mut self) {
		let idx = self.state.selected().map(|i| {
			if i == 0 {
				self.len.saturating_sub(1)
			} else {
				i.saturating_sub(1)
			}
		});
		self.state.select(idx);
	}

	fn pg_down(&mut self) {
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

	fn pg_up(&mut self) {
		if let Some(page) = self.page {
			let idx = self.state.selected().map(|i| i.saturating_sub(page));
			self.state.select(idx);
			*self.state.offset_mut() = self.state.offset().saturating_sub(page);
		}
	}

	fn home(&mut self) {
		self.state.select(Some(0));
		*self.state.offset_mut() = 0;
	}

	fn end(&mut self) {
		let len = self.len.saturating_sub(1);
		self.state.select(Some(len));
		*self.state.offset_mut() = self.offset();
	}

	fn enter(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let idx = self.state.selected().expect("state should always be Some");
		queue.select_idx(idx, player)
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

		let list = queue
			.path()
			.and_then(|path| lists.iter().find_map(|list| list.find(path)));

		let idx = if let Some(track) = queue.track()
			&& let Some(list) = &list
		{
			list.children()
				.iter()
				.enumerate()
				.find_map(|(i, child)| (child == track).then_some(i))
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

	fn len(&self) -> usize {
		if let Some(list) = &self.list {
			list.children().len()
		} else {
			self.lists.len()
		}
	}

	fn offset(&self) -> usize {
		self.page
			.map_or(usize::MAX, |page| self.len().saturating_sub(page))
	}

	fn curr(&self) -> ListType<'_> {
		if let Some(list) = &self.list {
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
}

impl Popup for Lists {
	fn draw(&mut self, frame: &mut Frame, area: Rect, queue: &Queue) {
		let children = self.list.as_ref().map(|list| list.children());
		let items = if let Some(children) = &children {
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

		let line = self.list.as_ref().map_or_else(
			|| utils::widgets::line("<< \"/\"", Style::default().bold()),
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

	fn change_track(&mut self, queue: &Queue) {
		let Some(track) = queue.track() else { return };
		if let Some(list) = &self.list {
			let children = list.children();
			let idx = children.iter().position(|child| child == track);
			let idx = idx.unwrap_or(0);

			self.state.select(Some(idx));
			*self.state.offset_mut() = self.offset();
		}
	}

	fn down(&mut self) {
		let max = self.len().saturating_sub(1);
		let idx = self
			.state
			.selected()
			.map(|i| if i == max { 0 } else { i.saturating_add(1) });

		self.state.select(idx);
	}

	fn up(&mut self) {
		let idx = self.state.selected().map(|i| {
			if i == 0 {
				self.len().saturating_sub(1)
			} else {
				i.saturating_sub(1)
			}
		});

		self.state.select(idx);
	}

	fn pg_down(&mut self) {
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

	fn pg_up(&mut self) {
		if let Some(page) = self.page {
			let idx = self.state.selected().map(|i| i.saturating_sub(page));
			self.state.select(idx);
			*self.state.offset_mut() = self.state.offset().saturating_sub(page);
		}
	}

	fn home(&mut self) {
		self.state.select(Some(0));
		*self.state.offset_mut() = 0;
	}

	fn end(&mut self) {
		let len = self.len().saturating_sub(1);
		self.state.select(Some(len));
		*self.state.offset_mut() = self.offset();
	}

	fn right(&mut self) {
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

	fn left(&mut self) {
		if let Some(list) = self.list.take() {
			if list.has_parent() {
				let (idx, parent) = list.into_parent().unwrap();
				self.set(Some(parent), idx.unwrap_or(0));
			} else {
				let idx = self.lists.iter().position(|root| root == &list);
				self.set(None, idx.unwrap_or(0));
			}
		}
	}

	fn enter(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let curr = self.curr();

		match curr {
			ListType::List(list) => {
				let list = list.clone();
				self.set(Some(list), 0);
			}
			ListType::Child(child, parent) => match child {
				Child::List(list) => {
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

	fn space(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let curr = self.curr();

		match curr {
			ListType::List(list) => {
				queue.queue(&list.path)?;
				queue.next(player);
			}
			ListType::Child(child, parent) => match child {
				Child::List(list) => {
					queue.queue(&list.path)?;
					queue.next(player);
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
