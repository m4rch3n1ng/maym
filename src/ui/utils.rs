use std::time::Duration;

pub fn fmt_duration(duration: Duration) -> String {
	let min = (duration.as_secs() / 60) % 60;
	let sec = duration.as_secs() % 60;

	format!("{:0>2}:{:0>2}", min, sec)
}
