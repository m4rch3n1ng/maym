use smol::future;
use std::{
	sync::mpsc::{Receiver, Sender, channel},
	time::Duration,
};
use zbus::{connection, interface};

struct MprisRoot;

// https://specifications.freedesktop.org/mpris-spec/2.2/Media_Player.html
#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
	#[zbus(property)]
	fn can_quit(&self) -> bool {
		false
	}

	#[zbus(property)]
	fn can_raise(&self) -> bool {
		false
	}

	#[zbus(property)]
	fn identity(&self) -> &'static str {
		"maym"
	}

	#[zbus(property)]
	fn supported_uri_schemes(&self) -> Vec<&str> {
		vec![]
	}

	#[zbus(property)]
	fn supported_mime_types(&self) -> Vec<&str> {
		vec![]
	}

	fn quit(&self) {}

	fn raise(&self) {}
}

struct MprisPlayer {
	tx: Sender<MprisEvent>,
}

// https://specifications.freedesktop.org/mpris-spec/2.2/Player_Interface.html
#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
	// #[zbus(property)]
	// fn playback_status(&self) -> &'static str {}

	#[zbus(property)]
	fn loop_status(&self) -> &'static str {
		"Playlist"
	}

	#[zbus(property)]
	fn rate(&self) -> f64 {
		1.0
	}

	#[zbus(property)]
	fn minimum_rate(&self) -> f64 {
		1.0
	}

	#[zbus(property)]
	fn maximim_rate(&self) -> f64 {
		1.0
	}

	// #[zbus(property)]
	// fn volume(&self) -> f64 {}

	// #[zbus(property)]
	// microseconds
	// fn position(&self) -> f64 {}

	#[zbus(property)]
	fn can_go_next(&self) -> bool {
		true
	}

	#[zbus(property)]
	fn can_go_previous(&self) -> bool {
		true
	}

	#[zbus(property)]
	fn can_play(&self) -> bool {
		true
	}

	#[zbus(property)]
	fn can_pause(&self) -> bool {
		true
	}

	#[zbus(property)]
	fn can_seek(&self) -> bool {
		true
	}

	#[zbus(property)]
	fn can_control(&self) -> bool {
		true
	}

	fn next(&self) {
		self.tx.send(MprisEvent::Next).unwrap();
	}

	fn previous(&self) {
		self.tx.send(MprisEvent::Prev).unwrap();
	}

	fn pause(&self) {
		self.tx.send(MprisEvent::Pause).unwrap();
	}

	fn play(&self) {
		self.tx.send(MprisEvent::Play).unwrap();
	}

	fn play_pause(&self) {
		self.tx.send(MprisEvent::Toggle).unwrap();
	}

	fn seek(&self, offset: i64) {
		let event = if offset < 0 {
			let offset = offset.unsigned_abs();
			let duration = Duration::from_micros(offset);
			MprisEvent::SeekBack(duration)
		} else {
			let duration = Duration::from_micros(offset as u64);
			MprisEvent::Seek(duration)
		};
		self.tx.send(event).unwrap();
	}
}

pub enum MprisEvent {
	Next,
	Prev,
	Toggle,
	Pause,
	Play,
	Seek(Duration),
	SeekBack(Duration),
}

#[derive(Debug)]
pub struct Mpris {
	rx: Receiver<MprisEvent>,
}

impl Mpris {
	pub fn new() -> Self {
		let (tx, rx) = channel::<MprisEvent>();

		let root = MprisRoot;
		let player = MprisPlayer { tx };

		smol::spawn(async {
			let _ = Mpris::serve(root, player).await;
		})
		.detach();

		// std::mem::forget(handle);

		Mpris { rx }
	}

	async fn serve(root: MprisRoot, player: MprisPlayer) -> Result<(), zbus::Error> {
		let connection = connection::Builder::session()?
			.name("org.mpris.MediaPlayer2.maym")?
			.serve_at("/org/mpris/MediaPlayer2", root)?
			.serve_at("/org/mpris/MediaPlayer2", player)?
			.build()
			.await?;

		std::mem::forget(connection);
		future::pending::<()>().await;

		Ok(())
	}

	pub fn recv(&self) -> Option<MprisEvent> {
		self.rx.try_recv().ok()
	}
}
