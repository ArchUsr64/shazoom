#![feature(stmt_expr_attributes)]

use std::io::Write;

use clap::Parser;
use log::{debug, error, info};

mod database;
mod encoder;

use crate::encoder::{Freq, TimeStamp};

#[derive(Parser, Clone)]
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
	#[arg(long, default_value_t = String::from("songs"))]
	pub songs_dir: String,
}

fn main() {
	env_logger::init();
	let args = Args::parse();
	let db_config = database::DatabaseConfig::from_args(args.clone());
	debug!("{db_config:?}");
	let mut db_builder = database::DatabaseBuilder::default();

	let entries = match std::fs::read_dir(&args.songs_dir) {
		Ok(x) => x,
		Err(err) => {
			error!("Failed to read songs directory {:?}", args.songs_dir);
			panic!("{err:?}")
		}
	};
	for dir_entry in entries {
		match dir_entry {
			Ok(file) => {
				if let Err(err) = db_builder.add_song(&file.path()) {
					error!("Failed to add {file:?} to the database, {err:?}");
				}
			}
			Err(err) => error!("{err}"),
		}
	}

	let start = std::time::Instant::now();
	let db = db_builder.build(db_config);
	info!("DB Build Took {:?}", start.elapsed());

	loop {
		let mut input_sample_path = String::new();
		print!("Enter file path: ");
		std::io::stdout().flush().unwrap();
		std::io::stdin().read_line(&mut input_sample_path).unwrap();
		let start = std::time::Instant::now();
		match std::fs::read(input_sample_path.trim()) {
			Ok(byte_array) => {
				let sample = encoder::Song::from_wav(byte_array);
				let mut matches = db.match_sample(sample);
				info!("Match Count: {}, in {:?}", matches.len(), start.elapsed());
				matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
				if let Some(best_match) = matches.first() {
					info!(
						"Best Match: {:?}, Score: {:.2}",
						db_builder.songs_path[best_match.id as usize], best_match.score
					);
					for (i, m) in matches.iter().enumerate() {
						debug!("{i}: Match: {m:?}");
					}
				}
			}
			Err(err) => {
				error!("Try again, {err:?}");
			}
		}
	}
}
