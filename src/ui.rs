use crate::state::State;
use ratatui::Frame;

#[derive(Debug, Default)]
pub struct Ui {}

impl Ui {
	#[allow(unused_variables)]
	#[allow(clippy::needless_pass_by_ref_mut)]
	pub fn draw(&mut self, frame: &mut Frame, state: &State) {
		// todo!

		println!("tick {:?}\r", state);
	}
}
