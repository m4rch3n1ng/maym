use std::time::Duration;

pub fn fmt_duration(duration: Duration) -> String {
	let min = (duration.as_secs() / 60) % 60;
	let sec = duration.as_secs() % 60;

	format!("{:0>2}:{:0>2}", min, sec)
}

pub mod widgets {
	use ratatui::{
		style::Style,
		text::{Line, Span},
	};
	use std::borrow::Cow;

	pub fn line<'a, I: Into<Cow<'a, str>>>(txt: I, style: Style) -> Line<'a> {
		let spans = vec![Span::styled(txt, style)];
		Line::from(spans)
	}
}

pub mod style {
	use crate::config::Config;
	use ratatui::style::{Color, Style, Stylize};
	use std::sync::OnceLock;

	static ACCENT: OnceLock<Color> = OnceLock::new();

	pub fn load(config: &Config) {
		if let Some(color) = config.accent() {
			ACCENT.set(color).expect("load should only be called once");
		}
	}

	pub fn accent() -> Style {
		let color = ACCENT.get().unwrap_or(&Color::Cyan);
		Style::new().fg(*color)
	}

	pub fn gauge_style(paused: bool) -> (Style, Style) {
		if paused {
			(accent().dim(), Style::new().dim())
		} else {
			(accent(), Style::new())
		}
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
