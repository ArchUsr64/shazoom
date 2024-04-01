//! Handles conversion of a WAV file on disk into a vector of Frequency signatures

use easyfft::prelude::DynRealFft;

#[derive(Debug, Clone)]
pub struct Song {
	pub sample_rate: usize,
	pub samples: Vec<f32>,
}
impl Song {
	pub fn from_wav(file_path: String) -> Song {
		let byte_array = std::fs::read(file_path).unwrap();
		let channel_count = u16::from_le_bytes([byte_array[22], byte_array[23]]);
		assert_eq!(channel_count, 1, "Only mono channel files are supported!");
		let sample_rate = u16::from_le_bytes([byte_array[24], byte_array[25]]) as usize;
		let mut samples = Vec::new();
		let mut byte_iter = byte_array.iter();
		while let Some(&lsb) = byte_iter.next() {
			if let Some(&msb) = byte_iter.next() {
				let word_data = i16::from_le_bytes([lsb, msb]);
				samples.push(word_data as f32 / i16::MAX as f32);
			}
		}
		Song {
			sample_rate,
			samples,
		}
	}

	/// For each time slice of duration `slice_size`, compute the frequency with the
	/// highest amplitude for each frequency bucket.
	///
	/// The frequency range spans from 0 to `bucket_size` * `bucket_count`
	pub fn constelation_map(
		&self,
		slice_size: std::time::Duration,
		bucket_size: usize,
		bucket_count: usize,
	) -> Vec<Vec<usize>> {
		let sample_window_size = self.sample_rate * slice_size.as_millis() as usize / 1000;
		let mut fft_extended_buffer = vec![0f32; self.sample_rate];
		self.samples
			.chunks_exact(sample_window_size)
			.map(|slice| {
				slice
					.iter()
					.zip(fft_extended_buffer.iter_mut())
					.for_each(|(&sample, buffer)| *buffer = sample);
				let freq_amplitudes: Vec<_> = fft_extended_buffer
					.real_fft()
					.iter()
					.map(|i| i.norm())
					.take(bucket_size * bucket_count)
					.enumerate()
					.collect();
				let mut bucket_frequencies: Vec<_> = freq_amplitudes
					.chunks_exact(bucket_size)
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
					.collect()
			})
			.collect()
	}
}
