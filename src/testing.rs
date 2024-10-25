use crate::encoder;
pub const snrs: [u8; 10] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
pub const offsets: [usize; 12] = [15, 30, 45, 60, 75, 90, 105, 120, 135, 150, 165, 180];

pub fn test() {
	let song = encoder::Song::from_wav(std::fs::read("test/song.wav").unwrap());
	let noise = encoder::Song::from_wav(std::fs::read("test/noise.wav").unwrap());
	for snr in snrs {
		for offset in offsets {
			let test_song = encoder::Song::mix(&song, &noise, snr as f32 / 100., offset, 15);
			std::fs::write(
				dbg!(format!("test/{}/{}.wav", snr, offset)),
				encoder::Song::to_wav(test_song),
			)
			.unwrap();
		}
	}
}
