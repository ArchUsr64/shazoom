//! Handles management of the song fingerprints

use std::collections::HashMap;

use crate::encoder;

const fn fuzz(signature: ((usize, usize), usize), fuzz_factor: usize) -> ((usize, usize), usize) {
	(
		(signature.0 .0 & !fuzz_factor, signature.0 .1 & !fuzz_factor),
		signature.1,
	)
}

#[derive(Clone, Copy, Debug)]
pub struct DatabaseConfig {
	slice_size: std::time::Duration,
	amplitude_normalization_smoothing_factor: u32,
	max_samples_per_slice: usize,
	bucket_size: usize,
	bucket_count: usize,
	target_zone_size: (usize, usize),
	fuzz_factor: usize,
}
impl Default for DatabaseConfig {
	fn default() -> Self {
		Self {
			slice_size: std::time::Duration::from_millis(100),
			amplitude_normalization_smoothing_factor: 10,
			max_samples_per_slice: 8,
			bucket_size: 100,
			bucket_count: 16,
			target_zone_size: (10, 400),
			fuzz_factor: 0b11,
		}
	}
}
impl DatabaseConfig {
	pub fn signatures(&self, song: encoder::Song) -> Vec<Vec<((usize, usize), usize)>> {
		let constellation_map = song.amplitude_normalized_constellation_map(
			self.slice_size,
			self.amplitude_normalization_smoothing_factor,
			self.max_samples_per_slice,
			self.bucket_size,
			self.bucket_count,
		);
		let signatures = encoder::Song::signatures(
			self.target_zone_size.0,
			self.target_zone_size.1,
			constellation_map,
		);
		signatures
			.iter()
			.map(|signatures| {
				signatures
					.iter()
					.map(|&signature| fuzz(signature, self.fuzz_factor))
					.collect()
			})
			.collect()
	}
}

#[derive(Default, Debug)]
pub struct DatabaseBuilder {
	songs_path: Vec<String>,
}
impl DatabaseBuilder {
	pub fn add_song(mut self, file_path: &'static str) -> Self {
		self.songs_path.push(file_path.into());
		self
	}
	pub fn build(&self, config: DatabaseConfig) -> Database {
		let mut db = Database::new(config);
		self.songs_path
			.iter()
			.enumerate()
			.for_each(|(song_id, song_path)| {
				let song = encoder::Song::from_wav(song_path);
				config
					.signatures(song)
					.iter()
					.enumerate()
					.for_each(|(timestamp, signature)| {
						signature
							.iter()
							.for_each(|&signature| db.add_signature(signature, song_id, timestamp))
					})
			});
		db
	}
}

#[derive(Debug)]
pub struct Database {
	data: HashMap<((usize, usize), usize), Vec<(usize, usize)>>,
	config: DatabaseConfig,
}
impl Database {
	pub fn new(config: DatabaseConfig) -> Self {
		Self {
			config,
			data: HashMap::new(),
		}
	}
	pub fn add_signature(
		&mut self,
		signature: ((usize, usize), usize),
		song_id: usize,
		timestamp: usize,
	) {
		if let Some(vec) = self.data.get_mut(&signature) {
			vec.push((song_id, timestamp))
		} else {
			self.data.insert(signature, vec![(song_id, timestamp)]);
		}
	}
	pub fn data(&self) -> &HashMap<((usize, usize), usize), Vec<(usize, usize)>> {
		&self.data
	}
	pub fn match_sample(&self, sample: encoder::Song) -> Vec<(usize, usize)> {
		let mut song_offsets: HashMap<usize, HashMap<isize, usize>> = HashMap::new();
		let mut insert_match = |song_id, offset| {
			if let Some(freq_table) = song_offsets.get_mut(song_id) {
				if let Some(offset_freq) = freq_table.get_mut(&offset) {
					*offset_freq += 1;
				} else {
					freq_table.insert(offset, 1);
				}
			} else {
				song_offsets.insert(*song_id, HashMap::from([(offset, 1)]));
			}
		};
		self.config.signatures(sample).iter().enumerate().for_each(
			|(sample_timestamp, signatures)| {
				signatures.iter().for_each(|i| {
					if let Some(matches) = self.data.get(i) {
						matches.iter().for_each(|(song_id, song_timestamp)| {
							insert_match(
								song_id,
								*song_timestamp as isize - sample_timestamp as isize,
							)
						})
					}
				})
			},
		);
		song_offsets
			.iter()
			.map(|(&song_id, offset_freq_table)| {
				(
					song_id,
					offset_freq_table
						.iter()
						.map(|i| i.1)
						.max()
						.copied()
						.unwrap(),
				)
			})
			.collect()
	}
}
