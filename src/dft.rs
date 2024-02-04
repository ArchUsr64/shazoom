use std::fmt::Debug;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Store signature for every sample window with this time interval
const SAMPLE_WINDOW_LENGTH_MS: usize = 100;

/// Minimum frequency to test
const MIN_FREQ: usize = 100;
/// Maximum frequency to test
const MAX_FREQ: usize = 900;

/// The entire `MIN_FREQ..MAX_FREQ` range is split into `FREQ_BUCKETS`,
/// within each bucket range, the most prominent frequency is stored
const FREQ_BUCKETS_COUNT: usize = 8;

/// AND the frequencies with this factor to account for background noise during
/// lookup
const FUZZ_FACTOR: usize = 0b0;

/// Represents the sound fingerprint of a slice
/// The n most prominent frequenies of each `FREQ_BUCKET` is stored as a single
/// byte in the u64 in native endianness
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Signature {
	pub data: u64,
}

impl Debug for Signature {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut buffer = "{".to_string();
		let bucket_size = (MAX_FREQ - MIN_FREQ) / FREQ_BUCKETS_COUNT;
		self.data
			.to_ne_bytes()
			.iter()
			.enumerate()
			.for_each(|(i, &byte)| {
				buffer.push_str(&format!(
					"[{}-{}]:{}, ",
					i * bucket_size + MIN_FREQ,
					(i + 1) * bucket_size + MIN_FREQ,
					byte as usize + i * bucket_size + MIN_FREQ,
				))
			});
		buffer.pop().unwrap();
		buffer.pop().unwrap();
		write!(f, "{}}}", buffer)
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Song {
	pub signatures: Vec<Signature>,
	title: String,
}

impl Song {
	/// Expects a .wav file name without the extension in the `songs` director
	pub fn from_name(title: &'static str) -> Option<Song> {
		let (sample_rate, buffer) = parse_song(format!("songs/{}.wav", title))?;
		let sample_window_width = sample_rate as usize * SAMPLE_WINDOW_LENGTH_MS / 1000;
		let sample_window_count = buffer.len() / sample_window_width;
		let signatures = (0..sample_window_count)
			.into_par_iter()
			.map(|window_index| {
				(
					window_index * sample_window_width,
					(window_index + 1) * sample_window_width,
				)
			})
			.map(|(sample_init, sample_end)| {
				let sample_window = &buffer[sample_init..sample_end];
				let bucket_size = (MAX_FREQ - MIN_FREQ) / FREQ_BUCKETS_COUNT;
				(0..FREQ_BUCKETS_COUNT)
					.map(move |bucket_index| {
						let bucket_range = (bucket_index * bucket_size + MIN_FREQ)
							..((bucket_index + 1) * bucket_size + MIN_FREQ);
						bucket_range
							.map(|test_freq| {
								fourier_transform(sample_window, sample_rate, test_freq as f32)
							})
							.enumerate()
							.max_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap())
							.unwrap()
					})
					.enumerate()
					.fold(0, |acc, (bucket_index, (freq, _freq_amplitude))| {
						acc | (((freq % bucket_size) & !FUZZ_FACTOR) << (bucket_index * 8))
					})
			})
			.map(|signature_data| Signature {
				data: signature_data as u64,
			})
			.collect();
		Some(Song {
			signatures,
			title: title.into(),
		})
	}
}

fn parse_song(file_path: String) -> Option<(f32, Vec<f32>)> {
	let byte_array = std::fs::read(file_path).ok()?;
	let channel_count = u16::from_le_bytes([*byte_array.get(22)?, *byte_array.get(23)?]);
	assert_eq!(channel_count, 1);
	let data_len = u32::from_le_bytes([
		*byte_array.get(40)?,
		*byte_array.get(41)?,
		*byte_array.get(42)?,
		*byte_array.get(43)?,
	]) as usize;
	let sample_rate = u16::from_le_bytes([*byte_array.get(24)?, *byte_array.get(25)?]);
	assert_eq!(sample_rate, 44100);
	Some((
		sample_rate as f32,
		// First 44 bytes are metatdata as per the WAV spec
		byte_array[44..44 + data_len]
			.array_chunks()
			.map(|&x| i16::from_le_bytes(x))
			.map(|x| x as f32 / i16::MAX as f32)
			.collect(),
	))
}

fn fourier_transform(data: &[f32], sample_rate: f32, test_frequency: f32) -> f32 {
	let scalar = 2. * PI * test_frequency / sample_rate as f32;
	use std::f32::consts::PI;
	let real_part: f32 = data
		.iter()
		.enumerate()
		.map(|(i, sample)| sample * (i as f32 * scalar).cos())
		.sum();
	let img_part: f32 = data
		.iter()
		.enumerate()
		.map(|(i, sample)| sample * (i as f32 * scalar).sin())
		.sum();
	// This would be the output of an actual DFT but we can optimize since we
	// only care about the relative strengths of the frequencies
	// (real_part.powi(2) + img_part.powi(2)).sqrt() / data.len() as f32
	real_part.powi(2) + img_part.powi(2)
}
