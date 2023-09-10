use crate::state::State;
use ratatui::{prelude::Backend, Frame};

#[derive(Debug, Default)]
pub struct Ui {}

impl Ui {
	pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, state: &State) {
		// todo!

		println!("tick {:?}\r", state);
	}
}
