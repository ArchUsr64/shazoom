mod encoder;

fn main() {
	let song = encoder::Song::from_wav("songs/tvari-hawaii-vacation-159069 (1).mp3.wav".into());
	let slice_size = std::time::Duration::from_millis(100);
	let constillation_map = song.amplitude_normalized_constellation_map(slice_size, 10, 8, 100, 16);
	let mut point_count = 0;
	for (i, slice) in constillation_map.iter().enumerate() {
		for freq in slice {
			point_count += 1;
			println!("{i}, {}", freq)
		}
	}
	let song_len = song.length().as_secs();
	dbg!(point_count);
	dbg!(point_count / song_len);
}
