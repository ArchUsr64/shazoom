use std::collections::HashMap;

use log::*;

use crate::encoder::{Signature, Song};

pub const CACHE_DIR: &'static str = "cache";
pub const SONGS_DIR: &'static str = "songs";

pub struct DatabaseBuilder {
	songs_list: Vec<&'static str>,
}

impl DatabaseBuilder {
	pub fn new() -> DatabaseBuilder {
		DatabaseBuilder { songs_list: vec![] }
	}
	pub fn add_song(mut self, name: &'static str) -> DatabaseBuilder {
		self.songs_list.push(name);
		self
	}
	pub fn build(self) -> Option<Database> {
		let map = HashMap::new();
		self.songs_list.iter().for_each(|song_name| {
			let cache_file = &format!("{CACHE_DIR}/{song_name}.json");
			if let Some(song) = find_cache(cache_file) {
				info!("Cache HIT for {song_name} at {cache_file}");
			} else {
				warn!("Cache MISS for {song_name}, File {cache_file} not found");
				if let Some(song) = Song::from_file(format!("{SONGS_DIR}/{song_name}.wav")) {
					let buffer = serde_json::to_string(&song).expect("Failed to serialize");
					std::fs::write(cache_file, buffer).expect("Failed to write cache file to disk");
					info!("Cache file for {song_name}, written to {cache_file}");
				} else {
					panic!("Failed to encode {}", song_name);
				}
			}
		});
		Some(Database {
			map,
			songs_list: self.songs_list,
		})
	}
}

pub struct Database {
	/// Map from signature to a tuple containing the index of the matching song
	/// `songs_list` and the slice index of the matched signature
	map: HashMap<Signature, Vec<(usize, usize)>>,
	songs_list: Vec<&'static str>,
}

fn find_cache(file_path: &String) -> Option<Song> {
	serde_json::from_str(&std::fs::read_to_string(file_path).ok()?).ok()?
}
