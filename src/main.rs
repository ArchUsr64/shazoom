mod encoder;
use easyfft::prelude::*;

fn main() {
	let song = encoder::Song::from_wav("songs/tvari-hawaii-vacation-159069 (1).mp3.wav".into());
	let constillation_map = song.constelation_map(std::time::Duration::from_millis(100), 100, 16);
	for (i, slice) in constillation_map.iter().enumerate() {
		for freq in &slice[..4] {
			println!("{i}, {freq}")
		}
	}
}
