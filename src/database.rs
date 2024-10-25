//! Handles management of the song fingerprints

use std::{
	ffi::OsString,
	hash::{DefaultHasher, Hash, Hasher},
	path::PathBuf,
};

use crate::Args;
use log::{error, info, warn};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::encoder::{self, Freq, Signature, TimeStamp};

pub type SongId = u32;
pub type Offset = i32;

#[derive(Clone, Copy, Debug, Hash, Serialize)]
pub struct DatabaseConfig {
	slice_size: std::time::Duration,
	freq_per_slice: usize,
	bucket_size: Freq,
	bucket_count: usize,
	target_zone_size: (TimeStamp, Freq),
}
impl DatabaseConfig {
	fn cached_dir_name(&self) -> OsString {
		let mut hasher = DefaultHasher::new();
		self.hash(&mut hasher);
		format!("{:016x}", hasher.finish()).into()
	}
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
		encoder::Song::signatures(
			self.target_zone_size.0,
			self.target_zone_size.1,
			constellation_map,
		)
	}
	pub fn from_args(
		Args {
			ms_timeslice_size: slice_size_ms,
			freq_per_slice,
			size_bucket: bucket_size,
			count_bucket: bucket_count,
			width_target_zone: target_zone_size_width,
			target_zone_height: target_zone_size_height,
			..
		}: Args,
	) -> Self {
		// TODO: input validation, like `assert!(freq_per_slice >= bucket_count)`
		Self {
			slice_size: std::time::Duration::from_millis(slice_size_ms),
			freq_per_slice,
			bucket_size,
			bucket_count,
			target_zone_size: (target_zone_size_width, target_zone_size_height),
		}
	}
}

#[derive(Debug, Hash)]
pub struct SongEntry {
	pub name: OsString,
	pub path: PathBuf,
}
impl SongEntry {
	fn cached_file_name(&self) -> OsString {
		let mut hasher = DefaultHasher::new();
		self.hash(&mut hasher);
		format!("{}-{:016x}.json", self.name.display(), hasher.finish()).into()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SongData(Vec<(Signature, TimeStamp)>);

#[derive(Debug)]
pub enum BuilderEntry {
	CachedData(OsString, SongData),
	Entry(SongEntry),
}

#[derive(Debug, Clone, Copy)]
pub enum CacheStatus {
	Hit,
	Miss,
}

#[derive(Debug)]
pub struct DatabaseBuilder {
	data: Vec<BuilderEntry>,
	config: DatabaseConfig,
	songs_dir: PathBuf,
	cache_dir: Option<PathBuf>,
}
impl DatabaseBuilder {
	pub fn new<T: Into<PathBuf> + std::fmt::Debug + Copy>(
		config: DatabaseConfig,
		songs_dir: T,
		cache_dir: Option<T>,
	) -> Self {
		let mut cache_dir_path = match cache_dir {
			None => {
				return Self {
					data: Vec::new(),
					config,
					songs_dir: songs_dir.into(),
					cache_dir: None,
				}
			}
			Some(x) => PathBuf::from(x.into()),
		};
		let cache_dir = {
			let db_cache_dir_name = &config.cached_dir_name();
			match std::fs::read_dir(&cache_dir_path) {
				Ok(mut cache_dir) => cache_dir
					.find_map(move |i| {
						i.ok()
							.filter(|i| i.file_name() == *db_cache_dir_name)
							.map(|i| i.path())
							.filter(|i| i.is_dir())
					})
					.or_else(|| {
						cache_dir_path.push(db_cache_dir_name);
						std::fs::create_dir(&cache_dir_path)
							.inspect_err(|err| {
								error!(
									"Failed to create db cache directory at {:?}, {err:?}",
									&cache_dir_path
								)
							})
							.map(|_| cache_dir_path.clone())
							.ok()
					}),
				Err(err) => {
					error!(
						"Failed to read cache directory {:?}, {err:?}",
						&cache_dir_path
					);
					None
				}
			}
		};
		if cache_dir.is_none() {
			warn!("Cache directory for specified {config:?} not found");
		}
		Self {
			data: Vec::new(),
			config,
			songs_dir: songs_dir.into(),
			cache_dir,
		}
	}
	pub fn add_song<T: Into<OsString> + Copy + std::fmt::Debug>(
		&mut self,
		file_path: T,
	) -> Option<CacheStatus> {
		let mut path = self.songs_dir.clone();
		path.push(file_path.into());
		if !path.clone().exists() {
			return None;
		}
		let entry = SongEntry {
			name: file_path.clone().into(),
			path,
		};
		if let Some(mut cached_file) = self.cache_dir.clone() {
			cached_file.push(entry.cached_file_name());
			match std::fs::read(cached_file) {
				Ok(cached_data) => match serde_json::from_slice(&cached_data) {
					Ok(x) => {
						self.data
							.push(BuilderEntry::CachedData(file_path.into(), x));
						return Some(CacheStatus::Hit);
					}
					Err(err) => {
						warn!("Failed to deserialize cache file for {file_path:?}, {err:?}")
					}
				},
				Err(err) => warn!("Failed to read cache file for {file_path:?}, {err:?}"),
			}
		}
		self.data.push(BuilderEntry::Entry(entry));
		Some(CacheStatus::Miss)
	}
	pub fn build(self, config: DatabaseConfig) -> Database {
		let mut db = Database::new(config);
		let song_signatures = |byte_array| -> SongData {
			let song = encoder::Song::from_wav(byte_array);
			let signatures = config.signatures(&song);
			// TODO: set an estimated initial capacity
			let mut res = Vec::new();
			signatures.enumerate().for_each(|(timestamp, signature)| {
				signature
					.iter()
					.copied()
					.for_each(|i| res.push((i, timestamp as TimeStamp)))
			});
			SongData(res)
		};
		if let Some(mut path) = self.cache_dir.clone() {
			path.push("config.json");
			std::fs::write(path, serde_json::to_string(&self.config).unwrap()).unwrap();
		}
		let data: Vec<(OsString, SongData)> = self
			.data
			.par_iter()
			.map(|entry| match entry {
				// TODO: cloning big chunks of data
				BuilderEntry::CachedData(path, data) => (path.clone(), data.clone()),
				BuilderEntry::Entry(entry) => {
					let data = song_signatures(std::fs::read(&entry.path).unwrap());
					if let Some(mut path) = self.cache_dir.clone() {
						path.push(&entry.cached_file_name());
						std::fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
						info!("Wrote data for {path:?} to Cache");
					}
					(entry.name.clone(), data)
				}
			})
			.collect();
		for (path, SongData(data)) in data {
			data.iter().copied().for_each(|(signature, timestamp)| {
				let vec = db.data.entry(signature).or_insert(Vec::new());
				vec.push((db.song_paths.len() as SongId, timestamp));
			});
			db.song_paths.push(path);
		}
		db
	}
}

#[allow(unused)]
#[derive(Clone, Copy, Debug)]
pub struct Match {
	pub id: SongId,
	pub score: f32,
	pub offset: f32,
	pub freq: usize,
	pub n: usize,
}

#[derive(Debug)]
pub struct Database {
	data: FxHashMap<Signature, Vec<(SongId, TimeStamp)>>,
	config: DatabaseConfig,
	song_paths: Vec<OsString>,
}
impl Database {
	pub fn song_name(&self, id: SongId) -> String {
		self.song_paths[id as usize].clone().into_string().unwrap()
	}
	pub fn new(config: DatabaseConfig) -> Self {
		Self {
			config,
			data: FxHashMap::default(),
			song_paths: Vec::new(),
		}
	}
	#[allow(unused)]
	pub fn data(&self) -> &FxHashMap<Signature, Vec<(SongId, TimeStamp)>> {
		&self.data
	}
	pub fn match_sample(&self, sample: encoder::Song) -> Vec<Match> {
		let mut song_offsets: FxHashMap<SongId, FxHashMap<Offset, usize>> = FxHashMap::default();
		self.config
			.signatures(&sample)
			.enumerate()
			.for_each(|(sample_timestamp, signatures)| {
				signatures.iter().for_each(|i| {
					if let Some(matches) = self.data.get(i) {
						matches.iter().for_each(|(song_id, song_timestamp)| {
							let offset = *song_timestamp as Offset - sample_timestamp as Offset;
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
				let mut sum = 0;
				let mut max_freq = usize::MIN;
				let mut best_offset = 0;
				let mut n = 0;
				for (offset, freq) in offset_freq_table.iter() {
					if *freq > max_freq {
						max_freq = *freq;
						best_offset = *offset;
					}
					sum += *freq;
					n += 1;
				}
				let average = sum as f32 / n as f32;
				Match {
					id: song_id,
					offset: best_offset as f32 * self.config.slice_size.as_secs_f32(),
					freq: max_freq,
					score: max_freq as f32 / average,
					n,
				}
			})
			.collect()
	}
}
