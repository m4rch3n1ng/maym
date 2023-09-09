use crate::state::State;
use ratatui::{prelude::Backend, Frame};

pub fn ui<B: Backend>(frame: &mut Frame<B>, state: &State) {
	// todo!

	println!("tick {:?}\r", state);
}
