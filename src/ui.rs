use self::popup::{Lists, Tracks};
use crate::{
	config::Config,
	player::Player,
	queue::{Queue, QueueError},
	state::State,
};
use ratatui::{Frame, layout::Rect};
use std::fmt::Debug;

mod popup;
pub mod utils;
mod window;

trait Popup {
	fn draw(&mut self, frame: &mut Frame, area: Rect, queue: &Queue);

	fn change_track(&mut self, active: bool, queue: &Queue);

	fn change_queue(&mut self, queue: &Queue) {
		let _ = queue;
	}

	fn up(&mut self);

	fn down(&mut self);

	fn left(&mut self) {}

	fn right(&mut self, queue: &Queue) {
		let _ = queue;
	}

	fn pg_up(&mut self) {}

	fn pg_down(&mut self) {}

	fn home(&mut self) {}

	fn end(&mut self) {}

	fn enter(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let _ = (player, queue);
		Ok(())
	}

	fn space(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		let _ = (player, queue);
		Ok(())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopupType {
	Tags = 0,
	Lyrics = 1,
	Tracks = 2,
	Lists = 3,
}

pub struct Ui {
	popups: [Box<dyn Popup>; 4],
	popup: Option<PopupType>,
}

impl Debug for Ui {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Ui")
			.field("popups", &[..])
			.field("popup", &self.popup)
			.finish()
	}
}

impl Ui {
	pub fn new(queue: &Queue, config: &Config) -> Self {
		Ui {
			popups: [
				Box::new(self::popup::tags()),
				Box::new(self::popup::lyrics()),
				Box::new(Tracks::new(queue)),
				Box::new(Lists::new(config, queue)),
			],
			popup: None,
		}
	}

	#[cfg(feature = "mpris")]
	pub fn draw_lock(&mut self, frame: &mut Frame, state: &std::sync::Mutex<State>, queue: &Queue) {
		let state = state.lock().unwrap();
		self.draw(frame, &state, queue);
	}

	pub fn draw(&mut self, frame: &mut Frame, state: &State, queue: &Queue) {
		let size = frame.area();
		let (window, seek) = window::layout(size);

		window::main(frame, window, state);
		window::seek(frame, seek, state);

		if let Some(popup) = self.popup {
			let area = window::popup(window);
			self.popups[popup as usize].draw(frame, area, queue);
		}
	}

	pub fn is_popup(&self) -> bool {
		self.popup.is_some()
	}

	pub fn is_selectable(&self) -> bool {
		matches!(self.popup, Some(PopupType::Tracks | PopupType::Lists))
	}

	pub fn change_track(&mut self, queue: &Queue) {
		for (idx, popup) in self.popups.iter_mut().enumerate() {
			let active = self.popup.is_some_and(|popup| popup as usize == idx);
			popup.change_track(active, queue);
		}
	}

	pub fn change_queue(&mut self, queue: &Queue) {
		for popup in &mut self.popups {
			popup.change_queue(queue);
		}
	}

	fn toggle(&mut self, popup: PopupType) {
		if self.popup == Some(popup) {
			self.popup = None;
		} else {
			self.popup = Some(popup);
		}
	}

	pub fn tags(&mut self) {
		self.toggle(PopupType::Tags);
	}

	pub fn lyrics(&mut self) {
		self.toggle(PopupType::Lyrics);
	}

	pub fn tracks(&mut self) {
		self.toggle(PopupType::Tracks);
	}

	pub fn lists(&mut self) {
		self.toggle(PopupType::Lists);
	}

	pub fn up(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].up();
	}

	pub fn down(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].down();
	}

	pub fn left(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].left();
	}

	pub fn right(&mut self, queue: &Queue) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].right(queue);
	}

	pub fn pg_up(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].pg_up();
	}

	pub fn pg_down(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].pg_down();
	}

	pub fn home(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].home();
	}

	pub fn end(&mut self) {
		let Some(popup) = self.popup else { return };
		self.popups[popup as usize].end();
	}

	pub fn enter(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		if let Some(popup) = self.popup {
			self.popups[popup as usize].enter(player, queue)
		} else {
			Ok(())
		}
	}

	pub fn space(&mut self, player: &mut Player, queue: &mut Queue) -> Result<(), QueueError> {
		if let Some(popup) = self.popup {
			self.popups[popup as usize].space(player, queue)
		} else {
			Ok(())
		}
	}

	pub fn esc(&mut self) {
		self.popup = None;
	}
}
