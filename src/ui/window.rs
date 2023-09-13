use super::utils;
use crate::state::State;
use conv::{ConvUtil, UnwrapOrSaturate};
use ratatui::{
	prelude::{Alignment, Constraint, Direction, Layout, Rect},
	style::{Color, Style, Stylize},
	symbols,
	text::{Line, Text},
	widgets::{Block, Borders, LineGauge, Padding, Paragraph},
	Frame,
};
use std::time::Duration;

pub fn main(frame: &mut Frame, area: Rect, state: &State) {
	let bold = Style::default().bold();
	let dim = Style::default().dim().italic();

	if let Some(track) = state.track.as_ref() {
		let title = track
			.title()
			.map_or(Line::styled("track has no title", dim), |title| {
				Line::styled(title, bold)
			});
		let artist = track
			.artist()
			.map_or(Line::styled("track has no artist", dim), Line::from);

		let text = vec![title, artist];
		let block = Block::default()
			.title("main")
			.borders(Borders::ALL)
			.padding(Padding::new(4, 4, 2, 2));
		let para = Paragraph::new(text).block(block);
		frame.render_widget(para, area);
	}
}

pub fn seek(frame: &mut Frame, area: Rect, state: &State) {
	if let Some((elapsed, duration)) = state.elapsed_duration() {
		let block = Block::default().title("seek").borders(Borders::ALL);
		frame.render_widget(block, area);

		let chunks = Layout::default()
			.constraints([Constraint::Max(1), Constraint::Max(1)])
			.vertical_margin(2)
			.horizontal_margin(2)
			.split(area);

		let seek = chunks[0];
		self::seek_seek(frame, (elapsed, duration), seek);

		let info = chunks[1];
		self::seek_info(frame, state, info);
	} else {
		let block = Block::default().title("seek").borders(Borders::ALL);
		frame.render_widget(block, area);
	}
}

fn seek_seek(frame: &mut Frame, (elapsed, duration): (Duration, Duration), area: Rect) {
	let fmt_elapsed = utils::fmt_duration(elapsed);
	let fmt_duration = utils::fmt_duration(duration);
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

fn seek_info(frame: &mut Frame, state: &State, area: Rect) {
	let vol = state.volume;
	let muted = state.muted;
	let paused = state.paused;
	let shuffle = state.shuffle;
	let text = format!(
		"shuffle: {} ~ paused: {} ~ muted: {} ~ vol {: >2}%",
		shuffle, paused, muted, vol
	);

	let block = Block::default().padding(Padding::new(2, 2, 0, 0));
	let par = Paragraph::new(text)
		.block(block)
		.alignment(Alignment::Right);
	frame.render_widget(par, area);
}

pub fn layout(size: Rect) -> (Rect, Rect) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([Constraint::Min(0), Constraint::Max(6)])
		.split(size);
	(chunks[0], chunks[1])
}

pub fn popup(main: Rect) -> Rect {
	let vert = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Percentage(10),
			Constraint::Percentage(80),
			Constraint::Percentage(10),
		])
		.split(main);

	Layout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Percentage(20),
			Constraint::Percentage(60),
			Constraint::Percentage(20),
		])
		.split(vert[1])[1]
}
