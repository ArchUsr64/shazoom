//! Handles management of the song fingerprints

use rustc_hash::FxHashMap;

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
				println!("{song_id}");
				let song = encoder::Song::from_wav(song_path);
				config
					.signatures(song)
					.iter()
					.enumerate()
					.for_each(|(timestamp, signature)| {
						signature.iter().for_each(|&signature| {
							let vec = db.data.entry(signature).or_insert(Vec::new());
							vec.push((song_id, timestamp as TimeStamp));
						})
					})
			});
		db
	}
}

#[derive(Debug)]
pub struct Database {
	data: FxHashMap<Signature, Vec<(usize, TimeStamp)>>,
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
	pub fn data(&self) -> &FxHashMap<Signature, Vec<(usize, TimeStamp)>> {
		&self.data
	}
	pub fn match_sample(&self, sample: encoder::Song) -> Vec<(usize, usize)> {
		let mut song_offsets: FxHashMap<usize, FxHashMap<i32, usize>> = FxHashMap::default();
		self.config.signatures(sample).iter().enumerate().for_each(
			|(sample_timestamp, signatures)| {
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
			},
		);
		song_offsets
			.iter()
			.map(|(&song_id, offset_freq_table)| {
				(
					song_id,
					offset_freq_table
						.iter()
						.max_by_key(|i| i.1)
						.map(|i| {
							println!("Offset: {i:?}");
							i.1
						})
						.copied()
						.unwrap(),
				)
			})
			.collect()
	}
}
