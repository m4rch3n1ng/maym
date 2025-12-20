use crate::{
	queue::{Queue, Track},
	state::State,
};
use cpal::{
	StreamConfig,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use creek::{ReadDiskStream, ReadStreamOptions, SeekMode, SymphoniaDecoder, read::ReadError};
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use std::{collections::VecDeque, convert::identity, fmt::Debug, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
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
	},
	Status(PlaybackStatus),
	Volume(f32),
	SeekTo(Duration),
}

enum FromProcess {
	Playhead(Duration),
	IsDone,
}

struct Process {
	stream: Option<Box<ReadDiskStream<SymphoniaDecoder>>>,
	buffer: VecDeque<f32>,
	stream_config: StreamConfig,
	resampler: Option<FastFixedIn<f32>>,
	resample_buffer_in: [Vec<f32>; 2],
	resample_buffer_out: [Vec<f32>; 2],

	// status
	status: PlaybackStatus,
	volume: f32,
	done: bool,

	// comm
	from_main_rx: Consumer<ToProcess>,
	to_main_tx: Producer<FromProcess>,
}

impl Process {
	pub fn new(
		stream_config: StreamConfig,
		from_main_rx: Consumer<ToProcess>,
		to_main_tx: Producer<FromProcess>,
	) -> Self {
		Process {
			stream: None,
			buffer: VecDeque::new(),
			stream_config,
			resampler: None,
			resample_buffer_in: [Vec::new(), Vec::new()],
			resample_buffer_out: [Vec::new(), Vec::new()],

			status: PlaybackStatus::Paused,
			volume: 0.45,
			done: false,

			from_main_rx,
			to_main_tx,
		}
	}

	pub fn process(&mut self, data: &mut [f32]) {
		while let Ok(msg) = self.from_main_rx.pop() {
			match msg {
				ToProcess::UseStream { stream, status } => {
					let duration = Process::playhead(&stream);
					let _ = self.to_main_tx.push(FromProcess::Playhead(duration));

					let cpal_sample_rate = self.stream_config.sample_rate;
					let stream_sample_rate = stream.info().sample_rate.unwrap();

					if cpal_sample_rate != stream_sample_rate {
						let ratio = f64::from(cpal_sample_rate) / f64::from(stream_sample_rate);
						let block_size = stream.block_size();

						let resampler = FastFixedIn::<f32>::new(
							ratio,
							1.0,
							PolynomialDegree::Linear,
							block_size,
							2,
						)
						.unwrap();

						let frames = resampler.output_frames_max();

						self.resample_buffer_in[0].resize(block_size, 0.0);
						self.resample_buffer_in[1].resize(block_size, 0.0);

						self.resample_buffer_out[0].resize(frames, 0.0);
						self.resample_buffer_out[1].resize(frames, 0.0);

						self.buffer.clear();
						self.buffer.reserve(frames * 2);

						self.resampler = Some(resampler);
					} else {
						self.buffer.clear();
						self.buffer.reserve(stream.block_size() * 2);
						self.resampler = None;
					}

					self.status = status;
					self.done = false;
					self.stream = Some(stream);
				}
				ToProcess::Status(status) => {
					self.status = status;
				}
				ToProcess::Volume(volume) => {
					debug_assert!((0.0..=1.0).contains(&volume));
					self.volume = volume;
				}
				ToProcess::SeekTo(duration) => {
					if let Some(stream) = &mut self.stream {
						let sample_rate = stream.info().sample_rate.unwrap();
						let frame = duration.as_secs_f64() * sample_rate as f64;
						stream.seek(frame as usize, SeekMode::Auto).unwrap();

						self.buffer.clear();

						let _ = self.to_main_tx.push(FromProcess::Playhead(duration));
					}
				}
			}
		}

		if let Some(stream) = &mut self.stream {
			if self.done || !stream.is_ready().is_ok_and(identity) {
				Self::silence(data);
				return;
			}

			if self.status == PlaybackStatus::Paused {
				Self::silence(data);
				return;
			}

			while self.buffer.len() < data.len() {
				let block_size = stream.block_size();
				let read_data = match stream.read(stream.block_size()) {
					Ok(read_data) => read_data,
					Err(ReadError::EndOfFile) => {
						self.done = true;
						let _ = self.to_main_tx.push(FromProcess::IsDone);
						Self::silence(data);
						return;
					}
					err @ Err(_) => err.unwrap(),
				};

				let ch1 = read_data.read_channel(0);
				let ch2 = read_data.read_channel(if read_data.num_channels() == 1 { 0 } else { 1 });

				if let Some(resampler) = &mut self.resampler {
					let [in_ch1, in_ch2] = &mut self.resample_buffer_in;

					let ch1 = if ch1.len() < block_size {
						in_ch1.clear();
						in_ch1.extend_from_slice(ch1);
						in_ch1.resize(block_size, 0.0);
						in_ch1
					} else {
						ch1
					};

					let ch2 = if ch2.len() < block_size {
						in_ch2.clear();
						in_ch2.extend_from_slice(ch2);
						in_ch2.resize(block_size, 0.0);
						in_ch2
					} else {
						ch2
					};

					let (_, out_len) = resampler
						.process_into_buffer(&[ch1, ch2], &mut self.resample_buffer_out, None)
						.unwrap();

					let [ch1, ch2] = &self.resample_buffer_out;

					for i in 0..out_len {
						self.buffer.push_back(ch1[i]);
						self.buffer.push_back(ch2[i]);
					}
				} else {
					for i in 0..read_data.num_frames() {
						self.buffer.push_back(ch1[i]);
						self.buffer.push_back(ch2[i]);
					}
				}
			}

			for sample in &mut *data {
				*sample = self.buffer.pop_front().unwrap();
			}

			// apply volume
			for sample in &mut *data {
				// mpv uses `pow(volume, 3)`
				*sample *= self.volume.powi(3);
			}

			let duration = Process::playhead(stream);
			let _ = self.to_main_tx.push(FromProcess::Playhead(duration));
		}
	}

	fn playhead<D: creek::Decoder>(stream: &ReadDiskStream<D>) -> Duration {
		let sample_rate = stream.info().sample_rate.unwrap();
		let playhead = stream.playhead() as f64 / sample_rate as f64;
		Duration::from_secs_f64(playhead)
	}

	fn silence(data: &mut [f32]) {
		for sample in data.iter_mut() {
			*sample = 0.;
		}
	}
}

pub struct Player {
	// state
	muted: bool,
	volume: u8,
	done: bool,
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
	pub fn new() -> Self {
		let (to_process_tx, from_main_rx) = RingBuffer::<ToProcess>::new(64);
		let (to_main_tx, from_process_rx) = RingBuffer::<FromProcess>::new(256);

		let host = cpal::default_host();
		let device = host.default_output_device().unwrap();

		let default_output_config = device.default_output_config().unwrap();
		let stream_config = StreamConfig::from(default_output_config);

		let mut process = Process::new(stream_config.clone(), from_main_rx, to_main_tx);

		let stream = device
			.build_output_stream(
				&stream_config,
				move |data: &mut [f32], _: &cpal::OutputCallbackInfo| process.process(data),
				|err| match err {
					cpal::StreamError::BufferUnderrun => {}
					_ => panic!("an error occured {err:?}"),
				},
				None,
			)
			.unwrap();

		stream.play().unwrap();
		std::mem::forget(stream);

		Player {
			muted: false,
			volume: 45,
			done: false,

			status: PlaybackStatus::Paused,
			elapsed: None,
			duration: None,

			to_process_tx,
			from_process_rx,
		}
	}

	pub fn with_state(queue: &Queue, state: &State) -> Self {
		let mut player = Player::new();
		player.state(queue, state);

		player
	}

	pub fn update(&mut self) {
		while let Ok(msg) = self.from_process_rx.pop() {
			match msg {
				FromProcess::Playhead(duration) => {
					self.elapsed = Some(duration);
				}
				FromProcess::IsDone => {
					self.done = true;
				}
			}
		}
	}

	fn state(&mut self, queue: &Queue, state: &State) {
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

			self.revive(track, start);
		}
	}

	fn revive(&mut self, track: &Track, start: Duration) {
		self.replace_inner(track, PlaybackStatus::Paused, start);
	}

	fn replace_inner(&mut self, track: &Track, status: PlaybackStatus, start: Duration) {
		let opts = ReadStreamOptions::default();

		let mut read_stream = ReadDiskStream::new(track.path(), 0, opts).unwrap();

		// seek to the specified position in the track
		let sample_rate = read_stream.info().sample_rate.unwrap();
		let start_frame = start.as_secs_f64() * sample_rate as f64;
		read_stream
			.seek(start_frame as usize, SeekMode::Auto)
			.unwrap();

		// wait until the buffer is filled before sending it to the process thread
		read_stream.block_until_ready().unwrap();

		let num_frames = read_stream.info().num_frames;
		let secs = num_frames as f64 / sample_rate as f64;
		self.duration = Some(Duration::from_secs_f64(secs));
		self.elapsed = Some(start);

		self.status = status;
		self.done = false;

		self.to_process_tx
			.push(ToProcess::UseStream {
				stream: Box::new(read_stream),
				status,
			})
			.unwrap();
	}

	pub fn done(&self) -> bool {
		self.duration.is_some() && self.done
	}

	pub fn seek(&mut self, position: Duration) {
		let _ = self.to_process_tx.push(ToProcess::SeekTo(position));
	}

	pub fn toggle(&mut self) {
		let status = self.status.invert();
		self.status = status;
		let _ = self.to_process_tx.push(ToProcess::Status(status));
	}

	#[cfg(feature = "mpris")]
	pub fn pause(&mut self, status: PlaybackStatus) {
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

	#[cfg(feature = "mpris")]
	pub fn set_volume(&mut self, vol: u8) {
		self.volume = vol;

		let _ = self
			.to_process_tx
			.push(ToProcess::Volume(vol as f32 / 100.));
	}
}

pub trait Playable {
	fn replace(&mut self, track: &Track);
}

impl Playable for Player {
	fn replace(&mut self, track: &Track) {
		self.replace_inner(track, PlaybackStatus::Play, Duration::ZERO);
	}
}
