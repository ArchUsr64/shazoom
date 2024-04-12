#![feature(stmt_expr_attributes)]

use std::io::Write;

use clap::Parser;
use log::{debug, error, info};

mod database;
mod encoder;

use crate::encoder::{Freq, TimeStamp};

#[derive(Parser)]
pub struct Args {
	#[arg(short, long, default_value_t = 260)]
	pub ms_timeslice_size: u64,
	#[arg(short, long, default_value_t = 8)]
	pub freq_per_slice: usize,
	#[arg(short, long, default_value_t = 180)]
	pub size_bucket: Freq,
	#[arg(short, long, default_value_t = 20)]
	pub count_bucket: usize,
	#[arg(short, long, default_value_t = 10)]
	pub width_target_zone: TimeStamp,
	#[arg(short, long, default_value_t = 900)]
	pub target_zone_height: Freq,
}

fn main() {
	env_logger::init();
	let start = std::time::Instant::now();
	let db_config = database::DatabaseConfig::from_args(Args::parse());
	debug!("{db_config:?}");
	let mut db_builder = database::DatabaseBuilder::default();

	#[rustfmt::skip]
{
	db_builder.add_song("songs/Charlie Puth - Attention [Official Video] [nfs8NYg7yQM].webm.wav");
	db_builder.add_song("songs/Sia - Cheap Thrills (Official Lyric Video) ft. Sean Paul [nYh-n7EOtMA].webm.wav");
	db_builder.add_song("songs/The Chainsmokers - Closer (Lyric) ft. Halsey [PT2_F-1esPk].webm.wav");
	db_builder.add_song("songs/Alan Walker - Faded [60ItHLz5WEA].webm.wav");
	db_builder.add_song("songs/Coldplay - Hymn For The Weekend (Official Video) [YykjpeuMNEk].webm.wav");
	db_builder.add_song("songs/Dua Lipa - New Rules (Official Music Video) [k2qgadSvNyU].webm.wav");
	db_builder.add_song("songs/Eminem - Not Afraid [j5-yKhDd64s].webm.wav");
	db_builder.add_song("songs/The Weeknd - Starboy ft. Daft Punk (Official Video) [34Na4j8AVgA].webm.wav");
	db_builder.add_song("songs/DJ Snake - Taki Taki ft. Selena Gomez, Ozuna, Cardi B (Official Music Video) [ixkoVwKQaJg].webm.wav");
	db_builder.add_song("songs/Pitbull - Timber (Official Video) ft. Ke$ha [hHUbLv4ThOo].webm.wav");
}

	let db = db_builder.build(db_config);
	info!("DB Build Took {:?}", start.elapsed());

	loop {
		let mut input_sample_path = String::new();
		print!("Enter file path: ");
		std::io::stdout().flush().unwrap();
		std::io::stdin().read_line(&mut input_sample_path).unwrap();
		let start = std::time::Instant::now();
		if let Some(sample) = encoder::Song::from_wav(&input_sample_path.trim().to_string()) {
			let mut matches = db.match_sample(sample);
			info!("Match Count: {}, in {:?}", matches.len(), start.elapsed());
			matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
			for res in &matches[..3.min(matches.len())] {
				info!(
					"Song: {}, Score: {:.2}",
					db_builder.songs_path[res.id as usize], res.score
				)
			}
		} else {
			error!("Invalid file name, try again");
		}
	}
}
