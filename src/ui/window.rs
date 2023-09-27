use super::utils;
use crate::state::State;
use conv::{ConvUtil, UnwrapOrSaturate};
use ratatui::{
	prelude::{Alignment, Constraint, Direction, Layout, Rect},
	style::{Style, Stylize},
	symbols,
	text::{Line, Span},
	widgets::{Block, Borders, LineGauge, Padding, Paragraph},
	Frame,
};
use std::time::Duration;

pub fn main(frame: &mut Frame, area: Rect, state: &State) {
	let bold = Style::default().bold();
	let dim = Style::default().dim();
	let dim_italic = dim.italic();

	let block = Block::default()
		.title(" main ")
		.borders(Borders::ALL)
		.padding(Padding::new(4, 4, 2, 2));

	if let Some(track) = state.track.as_ref() {
		let title = track
			.title()
			.map_or(Line::styled("track has no title", dim_italic), |title| {
				Line::styled(title, bold)
			});
		let artist = track
			.artist()
			.map_or(Line::styled("track has no artist", dim_italic), Line::from);

		let text = if let Some(album) = track.album() {
			let album = Line::styled(album, dim);
			vec![title, artist, album]
		} else {
			vec![title, artist]
		};

		let para = Paragraph::new(text).block(block);
		frame.render_widget(para, area);
	} else {
		let line = Line::styled("no track playing", dim_italic);
		let para = Paragraph::new(line).block(block.border_style(dim));
		frame.render_widget(para, area);
	}
}

pub fn seek(frame: &mut Frame, area: Rect, state: &State) {
	let block = Block::default().title(" seek ").borders(Borders::ALL);

	if let Some((elapsed, duration)) = state.elapsed_duration() {
		frame.render_widget(block, area);

		let chunks = Layout::default()
			.constraints([Constraint::Max(1), Constraint::Max(1)])
			.vertical_margin(2)
			.horizontal_margin(2)
			.split(area);

		let seek = chunks[0];
		self::seek_seek(frame, (elapsed, duration), state, seek);

		let info = chunks[1];
		self::seek_info(frame, state, info);
	} else {
		let dimmed = Style::default().dim();
		let dim = dimmed.italic();

		let padding = Padding::new(2, 0, 1, 0);
		let line = Line::styled("no track playing", dim);
		let para = Paragraph::new(line).block(block.padding(padding).border_style(dimmed));
		frame.render_widget(para, area);
	}
}

fn seek_seek(
	frame: &mut Frame,
	(elapsed, duration): (Duration, Duration),
	state: &State,
	area: Rect,
) {
	let fmt_elapsed = utils::fmt_duration(elapsed);
	let fmt_duration = utils::fmt_duration(duration);
	let text = Line::from(vec![
		if state.paused {
			Span::styled(&fmt_elapsed, Style::default().dim())
		} else {
			Span::raw(&fmt_elapsed)
		},
		Span::raw(" / "),
		Span::raw(&fmt_duration),
	]);

	let len = fmt_elapsed.len() + 3 + fmt_duration.len() + 4;
	let len = len.approx_as::<u16>().unwrap_or_saturate();
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints([Constraint::Max(len), Constraint::Min(0)])
		.split(area);

	let text_area = chunks[0];
	let block = Block::default().padding(Padding::new(2, 0, 0, 0));
	let par = Paragraph::new(text).block(block);
	frame.render_widget(par, text_area);

	let gauge_area = chunks[1];
	let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
	let block = Block::default().padding(Padding::new(0, 2, 0, 0));
	let gauge = LineGauge::default()
		.block(block)
		.label("")
		.gauge_style(if state.paused {
			utils::style::accent().dim()
		} else {
			utils::style::accent()
		})
		.line_set(symbols::line::THICK)
		.ratio(progress);
	frame.render_widget(gauge, gauge_area);
}

fn seek_info(frame: &mut Frame, state: &State, area: Rect) {
	let fmt_vol = format!("{: >3}%", state.volume);
	let (vol_str, vol) = if state.muted {
		(
			Span::styled("[mute]", utils::style::accent()),
			Span::styled(fmt_vol, Style::default().dim()),
		)
	} else {
		(Span::raw("[vol]:"), Span::raw(fmt_vol))
	};

	let paused = if state.paused {
		Span::styled("[stop]", Style::default().dim())
	} else {
		Span::styled("[play]", utils::style::accent())
	};

	let shuffle = if state.shuffle {
		Span::styled("[shuffle]", utils::style::accent())
	} else {
		Span::styled("[no shuffle]", Style::default().dim())
	};

	let block = Block::default().padding(Padding::new(2, 2, 0, 0));
	let line = Line::from(vec![
		shuffle,
		Span::raw(" ~ "),
		paused,
		Span::raw(" ~ "),
		vol_str,
		vol,
	]);
	let par = Paragraph::new(line)
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
			Constraint::Percentage(15),
			Constraint::Percentage(80),
			Constraint::Percentage(5),
		])
		.split(main);

	Layout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Percentage(15),
			Constraint::Percentage(70),
			Constraint::Percentage(15),
		])
		.split(vert[1])[1]
}
