use std::time::Duration;

pub fn fmt_duration(duration: Duration) -> String {
	let min = (duration.as_secs() / 60) % 60;
	let sec = duration.as_secs() % 60;

	format!("{:0>2}:{:0>2}", min, sec)
}

pub mod popup {
	use ratatui::{
		style::{Style, Stylize},
		widgets::{Block, Borders, Padding},
	};

	pub fn block() -> Block<'static> {
		Block::default()
			.borders(Borders::ALL)
			.border_style(Style::default().dim())
			.padding(Padding::new(2, 2, 1, 1))
	}
}
