use std::time::Duration;

pub fn fmt_duration(duration: Duration) -> String {
	let min = (duration.as_secs() / 60) % 60;
	let sec = duration.as_secs() % 60;

	format!("{:0>2}:{:0>2}", min, sec)
}

pub mod style {
	use ratatui::style::{Style, Stylize};

	pub fn accent() -> Style {
		Style::default().cyan()
	}
}

pub mod popup {
	use ratatui::{
		prelude::{Constraint, Direction, Layout, Rect},
		style::{Style, Stylize},
		widgets::{Block, Borders, Padding},
	};

	pub fn block() -> Block<'static> {
		Block::default()
			.borders(Borders::ALL)
			.border_style(Style::default().dim())
			.padding(Padding::new(2, 2, 1, 1))
	}

	pub fn double_layout(area: Rect) -> (Rect, Rect) {
		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Max(1), Constraint::Min(0)])
			.split(area);

		let title = layout[0];
		let list = layout[1];
		(title, list)
	}
}
