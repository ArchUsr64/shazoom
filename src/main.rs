#![feature(array_chunks)]

use serde_json::to_string;
mod encoder;

fn main() {
	let song = encoder::Song::from_name("500_hz").unwrap();
	let serialized = to_string(&song).unwrap();
	println!("{}, {}", serialized.len(), serialized);
	let song: encoder::Song = serde_json::from_str(&serialized).unwrap();
	println!("{song:#?}");
}
