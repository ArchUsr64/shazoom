#![feature(array_chunks)]

use rayon::prelude::*;

const MIN_FREQ: usize = 100;
const MAX_FREQ: usize = 5000;
const FREQ_SAMPLES: usize = 490;
const FREQ_CHUNK: usize = (MAX_FREQ - MIN_FREQ) / FREQ_SAMPLES;

const SAMPLE_RATE: usize = 44100;

fn main() {
	let buffer = parse_song("songs/500_hz.wav").unwrap();
	(0..FREQ_SAMPLES)
		.map(|i| i * FREQ_CHUNK + MIN_FREQ)
		.for_each(|i| println!("{i} {}", fourier_transform(&buffer, SAMPLE_RATE, i)));
}

fn parse_song(file_path: &'static str) -> Option<Vec<f32>> {
	let byte_array = std::fs::read(file_path).ok()?;
	Some(
		byte_array[44..]
			.array_chunks()
			.map(|&x| i16::from_le_bytes(x))
			.map(|x| x as f32 / i16::MAX as f32)
			.collect(),
	)
}

fn fourier_transform(data: &[f32], sample_rate: usize, test_frequency: usize) -> f32 {
	let sample_rate = sample_rate as f32;
	let test_frequency = test_frequency as f32;
	use std::f32::consts::TAU;
	let real_part: f32 = data
		.par_iter()
		.enumerate()
		.map(|(i, sample)| sample * (i as f32 * test_frequency * TAU / sample_rate).cos())
		.sum();
	let img_part: f32 = data
		.par_iter()
		.enumerate()
		.map(|(i, sample)| sample * (i as f32 * test_frequency * TAU / sample_rate).sin())
		.sum();
	(real_part.powi(2) + img_part.powi(2)).sqrt() / data.len() as f32
}
