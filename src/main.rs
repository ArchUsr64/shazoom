mod encoder;

fn main() {
	let song = encoder::Song::from_wav("songs/tvari-hawaii-vacation-159069 (1).mp3.wav".into());
	let slice_size = std::time::Duration::from_millis(100);
	let constillation_map = song.amplitude_normalized_constellation_map(slice_size, 10, 8, 100, 16);
	let signatures = encoder::Song::signatures(400, 10, constillation_map);
	dbg!(signatures.iter().map(|i| i.len()).sum::<usize>());
	println!("{}", signatures[signatures.len() / 2].len());
	println!("{signatures:#?}");
}
