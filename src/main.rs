#![feature(array_chunks)]

mod dft;

fn main() {
	let song = dft::Song::from_name("500_hz");
	println!("{:?}", song.unwrap());
}
