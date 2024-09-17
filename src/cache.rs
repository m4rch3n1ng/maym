use std::{collections::VecDeque, fs::File, path::Path};
use symphonia::core::{
	audio::SampleBuffer,
	codecs::Decoder,
	formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
	io::{MediaSourceStream, MediaSourceStreamOptions},
	meta::MetadataOptions,
	probe::Hint,
	units::Time,
};

struct Cache {
	format: Box<dyn FormatReader>,
	decoder: Box<dyn Decoder>,
	track_id: u32,

	cache: VecDeque<Vec<f32>>,

	current_subcache: usize,
	current_frame_in_subcache: usize,
	current_frame_in_file: usize,
}

impl Cache {
	pub fn new(path: &Path, num_caches: usize) -> Self {
		let format = symphonia::default::get_probe()
			.format(
				&Hint::new(),
				MediaSourceStream::new(
					Box::new(File::open(path).unwrap()),
					MediaSourceStreamOptions::default(),
				),
				&FormatOptions::default(),
				&MetadataOptions::default(),
			)
			.unwrap()
			.format;

		let track = format.default_track().unwrap();

		let decoder = symphonia::default::get_codecs()
			.make(
				&track.codec_params,
				&symphonia::core::codecs::DecoderOptions::default(),
			)
			.unwrap();

		let track_id = track.id;

		let mut ret = Cache {
			format,
			decoder,
			track_id,
			cache: VecDeque::with_capacity(num_caches),
			current_subcache: 0,
			current_frame_in_subcache: 0,
			current_frame_in_file: 0,
		};
		ret.init_cache(num_caches);
		ret
	}

	pub fn fill_data(&mut self, data: &mut [f32]) {
		for (i, sample) in data.iter_mut().enumerate() {
			let subcache_index = self.current_frame_in_subcache + i;
			*sample = self.cache[self.current_subcache + (subcache_index / self.cache[0].len())]
				[subcache_index % self.cache[0].len()];
		}

		self.current_frame_in_file += data.len();
		self.current_frame_in_subcache += data.len();
		self.current_subcache += self.current_frame_in_subcache / self.cache[0].len();
		self.current_frame_in_subcache %= self.cache[0].len();

		self.refresh_cache();
	}

	pub fn seek_to(&mut self, frame: usize) {
		if frame == self.current_frame_in_file {
			return;
		}

		let secs = frame as u64 / self.sample_rate() as u64;
		let frac = frame as f64 / self.sample_rate() as f64 - secs as f64;

		if self.current_frame_in_file < frame {
			// jump forward
			let sample_diff = frame - self.current_frame_in_file;
			let mut subcache_diff = sample_diff / self.cache[0].len();
			let mut new_subcache_index =
				sample_diff % self.cache[0].len() + self.current_frame_in_subcache;
			while new_subcache_index > self.cache[0].len() {
				new_subcache_index -= self.cache[0].len();
				subcache_diff += 1;
			}

			if subcache_diff + self.current_subcache >= self.cache.len() {
				self.format
					.seek(
						SeekMode::Accurate,
						SeekTo::Time {
							time: Time::new(secs, frac),
							track_id: None,
						},
					)
					.unwrap();

				let num_caches = self.cache.len();
				self.cache.clear();
				self.init_cache(num_caches);
			} else {
				self.current_subcache += subcache_diff;
				self.current_frame_in_subcache += new_subcache_index;
				self.refresh_cache();
			}
		} else {
			// jump backward
			let sample_diff = self.current_frame_in_file - frame;
			let mut subcache_diff = sample_diff / self.cache[0].len();
			let mut new_subcache_index = sample_diff % self.cache[0].len();
			while new_subcache_index < self.current_frame_in_subcache {
				new_subcache_index += self.cache[0].len() - self.current_frame_in_subcache;
				subcache_diff -= 1;
			}

			if subcache_diff > self.current_subcache {
				self.format
					.seek(
						SeekMode::Accurate,
						SeekTo::Time {
							time: Time::new(secs, frac),
							track_id: None,
						},
					)
					.unwrap();

				let num_caches = self.cache.len();
				self.cache.clear();
				self.init_cache(num_caches);
			} else {
				self.current_subcache -= subcache_diff;
				self.current_frame_in_subcache -= new_subcache_index;
			}
		}

		self.current_frame_in_file = frame;
	}

	pub fn current_frame(&self) -> usize {
		self.current_frame_in_file
	}

	pub fn song_end(&self) -> u64 {
		self.decoder.codec_params().n_frames.unwrap()
	}

	pub fn sample_rate(&self) -> u32 {
		self.decoder.codec_params().sample_rate.unwrap()
	}

	fn refresh_cache(&mut self) {
		while self.current_subcache >= self.cache.len() / 2 {
			self.cache.pop_front();
			if let Some(sub_cache) = self.decode_packet() {
				self.cache.push_back(sub_cache);
				self.current_subcache -= 1;
			}
		}
	}

	fn init_cache(&mut self, num_caches: usize) {
		self.current_subcache = 0;
		self.current_frame_in_subcache = 0;

		while self.cache.len() < num_caches {
			if let Some(sub_cache) = self.decode_packet() {
				self.cache.push_back(sub_cache);
			}
		}
	}

	fn decode_packet(&mut self) -> Option<Vec<f32>> {
		if let Ok(packet) = self.format.next_packet() {
			if packet.track_id() == self.track_id {
				return None;
			}

			match self.decoder.decode(&packet) {
				Ok(audio_buf) => {
					let spec = *audio_buf.spec();
					let duration = u64::try_from(audio_buf.capacity()).unwrap();
					let mut sample_buffer = SampleBuffer::<f32>::new(duration, spec);

					sample_buffer.copy_interleaved_ref(audio_buf);
					let mut sample_buffer = sample_buffer.samples().to_vec();
					sample_buffer.resize(
						self.decoder.codec_params().max_frames_per_packet.unwrap() as usize,
						0.0,
					);
					return Some(sample_buffer);
				}
				Err(err) => {
					panic!("{}", err);
				}
			}
		}
		Some(vec![
			0.0;
			self.decoder.codec_params().max_frames_per_packet.unwrap()
				as usize
		])
	}
}
