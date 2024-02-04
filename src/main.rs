#![feature(array_chunks)]

mod database;
mod encoder;

fn main() {
	env_logger::init();
	let _ = database::DatabaseBuilder::new().add_song("500_hz").build();
}
