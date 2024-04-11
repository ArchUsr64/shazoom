//! Handles management of the song fingerprints

use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::encoder::{self, Freq, Signature, TimeStamp};

pub type SongId = u32;

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
			slice_size: std::time::Duration::from_millis(50),
			freq_per_slice: 8,
			bucket_size: 120,
			bucket_count: 32,
			target_zone_size: (5, 600),
			fuzz_factor: 0b1,
		}
	}
}
impl DatabaseConfig {
	pub fn signatures<'a>(
		&'a self,
		song: &'a encoder::Song,
	) -> impl Iterator<Item = Vec<Signature>> + 'a {
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
		signatures.map(|signatures| {
			signatures
				.iter()
				.map(|&signature| signature.fuzz(self.fuzz_factor))
				.collect()
		})
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
		let song_signatures = |path| -> Vec<(Signature, TimeStamp)> {
			let song = encoder::Song::from_wav(path);
			let signatures = config.signatures(&song);
			// TODO: set an estimated initial capacity
			let mut res = Vec::new();
			signatures.enumerate().for_each(|(timestamp, signature)| {
				signature
					.iter()
					.copied()
					.for_each(|i| res.push((i, timestamp as TimeStamp)))
			});
			res
		};
		let data: Vec<Vec<(Signature, TimeStamp)>> = self
			.songs_path
			.par_iter()
			.map(|path| song_signatures(path))
			.collect();
		for (song_id, data) in data.iter().enumerate() {
			data.iter().copied().for_each(|(signature, timestamp)| {
				let vec = db.data.entry(signature).or_insert(Vec::new());
				vec.push((song_id as SongId, timestamp));
			})
		}
		db
	}
}

#[derive(Debug)]
pub struct Database {
	data: FxHashMap<Signature, Vec<(SongId, TimeStamp)>>,
	config: DatabaseConfig,
}
impl Database {
	pub fn new(config: DatabaseConfig) -> Self {
		Self {
			config,
			data: FxHashMap::default(),
		}
	}
	#[allow(unused)]
	pub fn data(&self) -> &FxHashMap<Signature, Vec<(SongId, TimeStamp)>> {
		&self.data
	}
	pub fn match_sample(&self, sample: encoder::Song) -> Vec<(SongId, usize)> {
		let mut song_offsets: FxHashMap<SongId, FxHashMap<i32, usize>> = FxHashMap::default();
		self.config
			.signatures(&sample)
			.enumerate()
			.for_each(|(sample_timestamp, signatures)| {
				signatures.iter().for_each(|i| {
					if let Some(matches) = self.data.get(i) {
						matches.iter().for_each(|(song_id, song_timestamp)| {
							let offset = *song_timestamp as i32 - sample_timestamp as i32;
							let freq_table =
								song_offsets.entry(*song_id).or_insert(FxHashMap::default());
							let offset_freq = freq_table.entry(offset).or_insert(0);
							*offset_freq += 1;
						})
					}
				})
			});
		song_offsets
			.iter()
			.map(|(&song_id, offset_freq_table)| {
				(
					song_id,
					offset_freq_table
						.iter()
						.max_by_key(|i| i.1)
						.map(|i| i.1)
						.copied()
						.unwrap(),
				)
			})
			.collect()
	}
}
