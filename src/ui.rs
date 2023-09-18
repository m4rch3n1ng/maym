use crate::state::State;
use ratatui::{prelude::Backend, Frame};

#[derive(Debug, Default)]
pub struct Ui {}

impl Ui {
	#[allow(unused_variables)]
	#[allow(clippy::needless_pass_by_ref_mut)]
	pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, state: &State) {
		// todo!

		println!("tick {:?}\r", state);
	}
}
