use crate::queue::{Queue, Track};
use crate::state::State;
use cpal::{
	traits::{DeviceTrait, HostTrait, StreamTrait},
	StreamConfig,
};
use creek::{ReadDiskStream, ReadStreamOptions, SeekMode, SymphoniaDecoder};
use rtrb::{Consumer, Producer, RingBuffer};
use std::{fmt::Debug, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackStatus {
	Paused,
	Play,
}

impl PlaybackStatus {
	fn invert(self) -> Self {
		match self {
			PlaybackStatus::Paused => PlaybackStatus::Play,
			PlaybackStatus::Play => PlaybackStatus::Paused,
		}
	}
}

enum ToProcess {
	UseStream {
		stream: Box<ReadDiskStream<SymphoniaDecoder>>,
		status: PlaybackStatus,
		frame: usize,
	},
	Status(PlaybackStatus),
	Volume(f32),
	SeekTo(usize),
}

enum FromProcess {
	Playhead(usize),
}

struct Process {
	stream: Option<Box<ReadDiskStream<SymphoniaDecoder>>>,

	// status
	status: PlaybackStatus,
	volume: f32,

	// comm
	from_main_rx: Consumer<ToProcess>,
	to_main_tx: Producer<FromProcess>,
}

impl Process {
	pub fn new(from_main_rx: Consumer<ToProcess>, to_main_tx: Producer<FromProcess>) -> Self {
		Process {
			stream: None,

			status: PlaybackStatus::Paused,
			volume: 0.45,

			from_main_rx,
			to_main_tx,
		}
	}

	pub fn process(&mut self, data: &mut [f32]) {
		while let Ok(msg) = self.from_main_rx.pop() {
			match msg {
				ToProcess::UseStream {
					mut stream,
					status,
					frame,
				} => {
					stream.seek(frame, SeekMode::Auto).unwrap();
					let _ = self
						.to_main_tx
						.push(FromProcess::Playhead(stream.playhead()));

					self.status = status;
					self.stream = Some(stream);
				}
				ToProcess::Status(status) => {
					self.status = status;
				}
				ToProcess::Volume(volume) => {
					debug_assert!((0.0..=1.0).contains(&volume));
					self.volume = volume;
				}
				ToProcess::SeekTo(frame) => {
					if let Some(stream) = &mut self.stream {
						stream.seek(frame, SeekMode::Auto).unwrap();

						let _ = self
							.to_main_tx
							.push(FromProcess::Playhead(stream.playhead()));
					}
				}
			}
		}

		if let Some(stream) = &mut self.stream {
			if !stream.is_ready().unwrap() {
				// stream not ready
				return;
			}

			if let PlaybackStatus::Paused = self.status {
				Self::silence(data);
				return;
			}

			let read_frames = data.len() / 2;
			let read_data = stream.read(read_frames).unwrap();

			if read_data.num_channels() == 1 {
				let ch = read_data.read_channel(0);

				for i in 0..read_data.num_frames() {
					data[i * 2] = ch[i];
					data[i * 2 + 1] = ch[i];
				}
			} else if read_data.num_channels() == 2 {
				let ch1 = read_data.read_channel(0);
				let ch2 = read_data.read_channel(1);

				for i in 0..read_data.num_frames() {
					data[i * 2] = ch1[i];
					data[i * 2 + 1] = ch2[i];
				}
			}

			// apply volume
			for sample in data.iter_mut() {
				*sample *= self.volume;
			}

			let _ = self
				.to_main_tx
				.push(FromProcess::Playhead(stream.playhead()));
		}
	}

	fn silence(data: &mut [f32]) {
		for sample in data.iter_mut() {
			*sample = 0.;
		}
	}
}

pub struct Player {
	stream_config: StreamConfig,

	// state
	muted: bool,
	volume: u8,
	status: PlaybackStatus,
	elapsed: Option<Duration>,
	duration: Option<Duration>,

	// comm
	to_process_tx: Producer<ToProcess>,
	from_process_rx: Consumer<FromProcess>,
}

impl Debug for Player {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Player").finish_non_exhaustive()
	}
}

impl Player {
	pub fn new() -> color_eyre::Result<Self> {
		let (to_process_tx, from_main_rx) = RingBuffer::<ToProcess>::new(64);
		let (to_main_tx, from_process_rx) = RingBuffer::<FromProcess>::new(256);

		let mut process = Process::new(from_main_rx, to_main_tx);

		let host = cpal::default_host();
		let device = host.default_output_device().unwrap();

		let default_output_config = device.default_output_config().unwrap();
		let stream_config = StreamConfig::from(default_output_config);
		let stream = device
			.build_output_stream(
				&stream_config,
				move |data: &mut [f32], _: &cpal::OutputCallbackInfo| process.process(data),
				|err| panic!("an error occured {:?}", err),
				None,
			)
			.unwrap();

		stream.play().unwrap();
		std::mem::forget(stream);

		let player = Player {
			stream_config,

			muted: false,
			volume: 45,

			status: PlaybackStatus::Paused,
			elapsed: None,
			duration: None,

			to_process_tx,
			from_process_rx,
		};
		Ok(player)
	}

	pub fn with_state(queue: &Queue, state: &State) -> color_eyre::Result<Self> {
		let mut player = Player::new()?;
		player.state(queue, state)?;

		Ok(player)
	}

	pub fn update(&mut self) {
		while let Ok(msg) = self.from_process_rx.pop() {
			match msg {
				FromProcess::Playhead(playhead) => {
					let secs = playhead as f64 / self.stream_config.sample_rate.0 as f64;
					let secs = Duration::from_secs_f64(secs);
					self.elapsed = Some(secs);
				}
			}
		}
	}

	fn state(&mut self, queue: &Queue, state: &State) -> color_eyre::Result<()> {
		self.volume = state.volume;

		let volume = if state.muted {
			0.
		} else {
			state.volume as f32 / 100.
		};
		let _ = self.to_process_tx.push(ToProcess::Volume(volume));

		if let Some(track) = queue.track() {
			let start = state.elapsed();
			let start = start.unwrap_or_default();

			self.revive(track, start)?;
		}

		Ok(())
	}

	fn revive(&mut self, track: &Track, start: Duration) -> color_eyre::Result<()> {
		self.replace_inner(track, PlaybackStatus::Paused, start);
		Ok(())
	}

	pub fn replace(&mut self, track: &Track) {
		self.replace_inner(track, PlaybackStatus::Play, Duration::ZERO);
	}

	fn replace_inner(&mut self, track: &Track, status: PlaybackStatus, start: Duration) {
		// these are the options used by the
		// player example of creek
		let opts = ReadStreamOptions {
			num_cache_blocks: 20,
			num_caches: 2,
			..ReadStreamOptions::default()
		};

		let mut read_stream =
			ReadDiskStream::<SymphoniaDecoder>::new(&track.path, 0, opts).unwrap();

		// cache the start of the file into cache with index `0`
		let _ = read_stream.cache(0, 0);
		read_stream.seek(0, SeekMode::default()).unwrap();

		// wait until the buffer is filled before sending it to the process thread
		read_stream.block_until_ready().unwrap();

		let num_frames = read_stream.info().num_frames;
		let secs = num_frames as f64 / self.stream_config.sample_rate.0 as f64;
		self.duration = Some(Duration::from_secs_f64(secs));
		self.status = status;

		let start_frame = start.as_secs_f32() * self.stream_config.sample_rate.0 as f32;

		self.to_process_tx
			.push(ToProcess::UseStream {
				stream: Box::new(read_stream),
				status,
				frame: start_frame as usize,
			})
			.unwrap();
	}

	pub fn seek(&mut self, position: Duration) {
		self.elapsed = Some(position);
		let frame = position.as_secs_f32() * self.stream_config.sample_rate.0 as f32;
		let _ = self.to_process_tx.push(ToProcess::SeekTo(frame as usize));
	}

	pub fn toggle(&mut self) {
		let status = self.status.invert();
		self.status = status;
		let _ = self.to_process_tx.push(ToProcess::Status(status));
	}

	pub fn volume(&self) -> u8 {
		self.volume
	}

	pub fn paused(&self) -> bool {
		self.status == PlaybackStatus::Paused
	}

	pub fn duration(&self) -> Option<Duration> {
		self.duration
	}

	pub fn elapsed(&self) -> Option<Duration> {
		self.elapsed
	}

	pub fn mute(&mut self) {
		let muted = !self.muted;
		self.muted = muted;

		let vol = if muted { 0. } else { self.volume as f32 / 100. };
		let _ = self.to_process_tx.push(ToProcess::Volume(vol));
	}

	pub fn muted(&self) -> bool {
		self.muted
	}

	pub fn i_vol(&mut self, amt: u8) {
		let vol = u8::min(100, self.volume.saturating_add(amt));
		self.volume = vol;

		let _ = self
			.to_process_tx
			.push(ToProcess::Volume(vol as f32 / 100.));
	}

	pub fn d_vol(&mut self, amt: u8) {
		let vol = self.volume.saturating_sub(amt);
		self.volume = vol;

		let _ = self
			.to_process_tx
			.push(ToProcess::Volume(vol as f32 / 100.));
	}
}
