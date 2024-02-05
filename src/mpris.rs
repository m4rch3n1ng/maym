use crate::state::State;
use smol::future;
use std::{
	collections::HashMap,
	sync::{
		Arc, Mutex,
		mpsc::{Receiver, Sender, channel},
	},
	time::Duration,
};
use zbus::{connection, interface, zvariant::Value};

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
	state: Arc<Mutex<State>>,
}

// https://specifications.freedesktop.org/mpris-spec/2.2/Player_Interface.html
#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
	#[zbus(property)]
	fn playback_status(&self) -> &'static str {
		let state = self.state.lock().unwrap();
		if state.track.is_none() {
			"Stopped"
		} else if state.paused {
			"Paused"
		} else {
			"Playing"
		}
	}

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

	#[zbus(property)]
	fn shuffle(&self) -> bool {
		let state = self.state.lock().unwrap();
		state.shuffle
	}

	#[zbus(property)]
	fn set_shuffle(&self, shuffle: bool) {
		self.tx.send(MprisEvent::Shuffle(shuffle)).unwrap();
	}

	#[zbus(property)]
	fn metadata(&self) -> HashMap<&str, Value<'_>> {
		let state = self.state.lock().unwrap();
		let mut map = HashMap::new();

		if let Some(duration) = state.duration() {
			let duration = duration.as_micros() as u64;
			map.insert("mpris:length", Value::U64(duration));
		}

		if let Some(track) = &state.track {
			if let Some(album) = track.album().map(ToOwned::to_owned) {
				map.insert("xesam:album", Value::Str(album.into()));
			}

			if let Some(artist) = track.artist().map(ToOwned::to_owned) {
				map.insert("xesam:artist", Value::Str(artist.into()));
			}

			if let Some(title) = track.title().map(ToOwned::to_owned) {
				map.insert("xesam:title", Value::Str(title.into()));
			}

			if let Some(track) = track.track() {
				map.insert("xesam:discNumber", Value::U32(track));
			}
		}

		map
	}

	#[zbus(property)]
	fn volume(&self) -> f64 {
		let state = self.state.lock().unwrap();

		if state.muted {
			0.0
		} else {
			state.volume as f64 / 100.0
		}
	}

	#[zbus(property)]
	fn set_volume(&self, vol: f64) {
		if vol.is_nan() {
			return;
		}

		let vol = vol.clamp(0.0, 1.0);
		let vol = vol * 100.0;
		self.tx.send(MprisEvent::Volume(vol as u8)).unwrap();
	}

	#[zbus(property)]
	fn position(&self) -> i64 {
		let state = self.state.lock().unwrap();
		state.elapsed().unwrap_or(Duration::ZERO).as_micros() as i64
	}

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
	Shuffle(bool),
	Volume(u8),
}

#[derive(Debug)]
pub struct Mpris {
	rx: Receiver<MprisEvent>,
}

impl Mpris {
	pub fn new(state: Arc<Mutex<State>>) -> Self {
		let (tx, rx) = channel::<MprisEvent>();

		let root = MprisRoot;
		let player = MprisPlayer { tx, state };

		smol::spawn(async {
			let _ = Mpris::serve(root, player).await;
		})
		.detach();

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
