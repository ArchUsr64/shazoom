//! Handles management of the song fingerprints

use std::collections::HashMap;

use crate::encoder::{self, Freq, Signature, TimeStamp};

#[derive(Clone, Copy, Debug)]
pub struct DatabaseConfig {
	slice_size: std::time::Duration,
	freq_per_slice: usize,
	bucket_size: Freq,
	bucket_count: usize,
	target_zone_size: (TimeStamp, Freq),
	fuzz_factor: Freq,
}
impl Default for DatabaseConfig {
	fn default() -> Self {
		Self {
			slice_size: std::time::Duration::from_millis(100),
			freq_per_slice: 16,
			bucket_size: 100,
			bucket_count: 16,
			target_zone_size: (10, 400),
			fuzz_factor: 0b11,
		}
	}
}
impl DatabaseConfig {
	pub fn signatures(&self, song: encoder::Song) -> Vec<Vec<Signature>> {
		let constellation_map = song.constellation_map(
			self.slice_size,
			self.freq_per_slice,
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
					.map(|&signature| signature.fuzz(self.fuzz_factor))
					.collect()
			})
			.collect()
	}
}

#[derive(Default, Debug)]
pub struct DatabaseBuilder {
	pub songs_path: Vec<String>,
}
impl DatabaseBuilder {
	pub fn add_song(&mut self, file_path: &str) {
		self.songs_path.push(file_path.into());
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
						signature.iter().for_each(|&signature| {
							db.add_signature(signature, song_id, timestamp as TimeStamp)
						})
					})
			});
		db
	}
}

#[derive(Debug)]
pub struct Database {
	data: HashMap<Signature, Vec<(usize, TimeStamp)>>,
	config: DatabaseConfig,
}
impl Database {
	pub fn new(config: DatabaseConfig) -> Self {
		Self {
			config,
			data: HashMap::new(),
		}
	}
	pub fn add_signature(&mut self, signature: Signature, song_id: usize, timestamp: TimeStamp) {
		if let Some(vec) = self.data.get_mut(&signature) {
			vec.push((song_id, timestamp))
		} else {
			self.data.insert(signature, vec![(song_id, timestamp)]);
		}
	}
	#[allow(unused)]
	pub fn data(&self) -> &HashMap<Signature, Vec<(usize, TimeStamp)>> {
		&self.data
	}
	pub fn match_sample(&self, sample: encoder::Song) -> Vec<(usize, usize)> {
		let mut song_offsets: HashMap<usize, HashMap<i32, usize>> = HashMap::new();
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
							insert_match(song_id, *song_timestamp as i32 - sample_timestamp as i32)
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