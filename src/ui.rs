use crate::state::State;
use conv::{ConvUtil, UnwrapOrSaturate};
use ratatui::{
	prelude::{Alignment, Constraint, Direction, Layout, Rect},
	style::{Color, Style},
	symbols,
	text::Text,
	widgets::{Block, Borders, LineGauge, Padding, Paragraph},
	Frame,
};
use std::time::Duration;

#[derive(Debug, Default)]
pub struct Ui {}

impl Ui {
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		let size = frame.size();
		let (window, seek) = self.layout(size);

		self.draw_main(frame, window, state);
		self.draw_seek(frame, seek, state);
	}

	fn draw_main(&self, frame: &mut Frame, area: Rect, state: &State) {
		let text = state
			.track
			.as_ref()
			.map(ToString::to_string)
			.unwrap_or_default();
		let block = Block::default().title("main").borders(Borders::ALL);
		let para = Paragraph::new(text).block(block);
		frame.render_widget(para, area);
	}

	fn draw_seek(&self, frame: &mut Frame, area: Rect, state: &State) {
		if let Some((elapsed, duration)) = state.elapsed_duration() {
			let block = Block::default().title("seek").borders(Borders::ALL);
			frame.render_widget(block, area);

			let chunks = Layout::default()
				.constraints([Constraint::Max(1), Constraint::Max(1)])
				.vertical_margin(2)
				.horizontal_margin(2)
				.split(area);

			let seek = chunks[0];
			self.seek_seek(frame, (elapsed, duration), seek);

			let info = chunks[1];
			self.seek_info(frame, state, info);
		} else {
			let block = Block::default().title("seek").borders(Borders::ALL);
			frame.render_widget(block, area);
		}
	}

	fn seek_seek(&self, frame: &mut Frame, (elapsed, duration): (Duration, Duration), area: Rect) {
		let fmt_elapsed = fmt_duration(elapsed);
		let fmt_duration = fmt_duration(duration);
		let fmt = format!("{} / {}", fmt_elapsed, fmt_duration);

		let len = fmt.len() + 4;
		let len = len.approx_as::<u16>().unwrap_or_saturate();
		let chunks = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Max(len), Constraint::Min(0)])
			.split(area);

		let t = chunks[0];
		let text = Text::from(fmt);
		let block = Block::default().padding(Padding::new(2, 0, 0, 0));
		let par = Paragraph::new(text).block(block);
		frame.render_widget(par, t);

		let g = chunks[1];
		let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
		let block = Block::default().padding(Padding::new(0, 2, 0, 0));
		let gauge = LineGauge::default()
			.block(block)
			.label("")
			.gauge_style(Style::default().fg(Color::Magenta))
			.line_set(symbols::line::THICK)
			.ratio(progress);
		frame.render_widget(gauge, g);
	}

	fn seek_info(&self, frame: &mut Frame, state: &State, area: Rect) {
		let vol = state.volume;
		let muted = state.muted;
		let paused = state.paused;
		let text = format!("paused: {} ~ muted: {} ~ vol {: >2}%", paused, muted, vol);

		let block = Block::default().padding(Padding::new(2, 2, 0, 0));
		let par = Paragraph::new(text)
			.block(block)
			.alignment(Alignment::Right);
		frame.render_widget(par, area);
	}

	fn layout(&self, size: Rect) -> (Rect, Rect) {
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(0), Constraint::Max(6)])
			.split(size);
		(chunks[0], chunks[1])
	}
}

fn fmt_duration(duration: Duration) -> String {
	let min = (duration.as_secs() / 60) % 60;
	let sec = duration.as_secs() % 60;

	format!("{:0>2}:{:0>2}", min, sec)
}
