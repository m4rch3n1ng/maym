use crate::state::State;
use ratatui::{prelude::Backend, Frame};

#[derive(Debug)]
pub struct Tui {}

impl Tui {
	pub fn new() -> Self {
		Tui {}
	}

	pub fn ui<B: Backend>(&self, frame: &mut Frame<B>, state: &State) {
		// todo!

		println!("tick {:?}\r", state);
	}
}
