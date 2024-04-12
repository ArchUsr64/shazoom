//! Handles conversion of a WAV file on disk into a vector of Frequency signatures

use std::hash::Hash;

use easyfft::prelude::*;
use serde::{Deserialize, Serialize};

pub type Freq = u16;
pub type TimeStamp = u32;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature((Freq, Freq), TimeStamp);

#[derive(Debug, Clone)]
pub struct Song {
	pub sample_rate: usize,
	// TODO: will i16 samples work here??
	pub samples: Vec<f32>,
}
impl Song {
	pub fn from_wav(byte_array: Vec<u8>) -> Song {
		let channel_count = u16::from_le_bytes([byte_array[22], byte_array[23]]);
		assert_eq!(channel_count, 1, "Only mono channel files are supported!");
		let sample_rate = u16::from_le_bytes([byte_array[24], byte_array[25]]) as usize;
		let mut samples = Vec::with_capacity(byte_array.len() / 2);
		let mut byte_iter = byte_array.iter();
		while let Some(&lsb) = byte_iter.next() {
			if let Some(&msb) = byte_iter.next() {
				let word_data = i16::from_le_bytes([lsb, msb]);
				samples.push(word_data as f32);
			}
		}
		Song {
			sample_rate,
			samples,
		}
	}
	#[allow(unused)]
	pub fn length(&self) -> std::time::Duration {
		std::time::Duration::from_millis((self.samples.len() * 1000 / self.sample_rate) as u64)
	}

	pub fn signatures<'a, T: ExactSizeIterator<Item = Vec<Freq>>>(
		// This should be a `std::time::Duration`
		target_zone_width: TimeStamp,
		target_zone_height: Freq,
		constellation_map: T,
	) -> impl Iterator<Item = Vec<Signature>> + 'a {
		let constellation_map: Vec<_> = constellation_map.collect();
		(0..constellation_map.len() - 1).map(move |i| {
			let slice = &constellation_map[i];
			let target_slices =
				&constellation_map[i..constellation_map.len().min(i + target_zone_width as usize)];
			slice
				.iter()
				.copied()
				.map(move |anchor_freq| {
					target_slices
						.iter()
						.enumerate()
						.skip(1)
						.map(move |(time_offset, target_slice)| {
							target_slice
								.iter()
								.copied()
								.filter(move |target_freq| {
									(anchor_freq.saturating_sub(target_zone_height / 2)
										..anchor_freq + target_zone_height / 2)
										.contains(target_freq)
								})
								.map(move |target_freq| {
									Signature((anchor_freq, target_freq), time_offset as TimeStamp)
								})
						})
						.flatten()
				})
				.flatten()
				.collect()
		})
	}

	/// For each time slice of duration `slice_size`, compute the frequency with the
	/// highest amplitude for each frequency bucket.
	///
	/// The frequency range spans from 0 to `bucket_size` * `bucket_count`
	pub fn constellation_map(
		&self,
		slice_size: std::time::Duration,
		freq_per_slice: usize,
		bucket_size: Freq,
		bucket_count: usize,
	) -> impl ExactSizeIterator<Item = Vec<Freq>> + '_ {
		let sample_window_size = self.sample_rate * slice_size.as_millis() as usize / 1000;
		let mut fft_extended_buffer = vec![0f32; self.sample_rate];
		self.samples
			.chunks_exact(sample_window_size)
			.map(move |slice| {
				slice
					.iter()
					.zip(fft_extended_buffer.iter_mut())
					.for_each(|(&sample, buffer)| *buffer = sample);
				let freq_amplitudes: Vec<_> = fft_extended_buffer
					.real_fft()
					.iter()
					.map(|i| i.norm())
					.take(Into::<usize>::into(bucket_size) * bucket_count)
					.enumerate()
					.map(|(freq, ampl)| (freq as Freq, ampl))
					.collect();
				let mut bucket_frequencies: Vec<_> = freq_amplitudes
					.chunks_exact(bucket_size.into())
					.map(|freq_bucket| {
						freq_bucket
							.iter()
							.max_by(|(_freq_1, ampl_1), (_freq_2, ampl_2)| {
								ampl_1.partial_cmp(ampl_2).unwrap()
							})
							.unwrap()
					})
					.collect();
				bucket_frequencies.sort_unstable_by(|(_freq_1, ampl_1), (_freq_2, ampl_2)| {
					ampl_2.partial_cmp(ampl_1).unwrap()
				});
				bucket_frequencies
					.iter()
					.map(|(freq, _ampl)| *freq)
					.take(freq_per_slice)
					.collect()
			})
	}
}
