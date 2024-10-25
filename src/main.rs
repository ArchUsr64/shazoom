#![feature(stmt_expr_attributes, let_chains, os_str_display)]

use std::io::Write;

use clap::Parser;
use log::{debug, error, info};

mod database;
mod encoder;
mod testing;

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
	#[arg(long, default_value_t = String::from("test"))]
	pub songs_dir: String,
	#[arg(long, default_value_t = String::from("cache"))]
	pub cache_dir: String,
}

fn main() {
	// testing::test();
	// std::process::exit(0);
	env_logger::init();
	let args = Args::parse();
	let db_config = database::DatabaseConfig::from_args(args.clone());
	debug!("{db_config:?} at {:?}", db_config.cached_dir_name());
	let mut db_builder =
		database::DatabaseBuilder::new(db_config, &args.songs_dir, Some(&args.cache_dir));

	let entries = match std::fs::read_dir(&args.songs_dir) {
		Ok(x) => x,
		Err(err) => {
			error!("Failed to read songs directory {:?}", args.songs_dir);
			panic!("{err:?}")
		}
	};

	for dir_entry in entries {
		match dir_entry {
			Ok(x) if !x.path().is_file() => {
				debug!("Skipping {x:?}");
			}
			Ok(file) => match db_builder.add_song(&file.file_name()) {
				Some(cache) => info!("{cache:?} for {file:?}"),
				None => error!("Failed to add {file:?} to the database"),
			},
			Err(err) => error!("{err}"),
		}
	}

	let start = std::time::Instant::now();
	let db = db_builder.build(db_config);
	info!("DB Build Took {:?}", start.elapsed());

	for snr in testing::snrs.iter().rev() {
		let mut accuracies = Vec::new();
		for offset in testing::offsets {
			let mut input_sample_path = format!("test/{}/{}.wav", snr, offset);
			// print!("Enter file path: ");
			// std::io::stdout().flush().unwrap();
			// std::io::stdin().read_line(&mut input_sample_path).unwrap();
			let start = std::time::Instant::now();
			match std::fs::read(input_sample_path.trim()) {
				Ok(byte_array) => {
					let sample = encoder::Song::from_wav(byte_array);
					let mut matches = db.match_sample(sample);
					matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
					if let Some(best_match) = matches.first() {
						let mut total_score = 0.;
						println!(
							"Match for {input_sample_path}, is {}",
							db.song_name(best_match.id)
						);
						if best_match.id != 0 {
							error!("Match failed!");
							accuracies.push(0.);
							continue;
						}
						info!(
							"Best Match: {:?}, Score: {:.2}",
							db.song_name(best_match.id),
							best_match.score
						);
						for (i, m) in matches.iter().enumerate() {
							debug!("{i}: Match: {m:?}");
							total_score += m.score;
						}
						accuracies.push(best_match.score / total_score);
					}
					info!("Match Count: {}, in {:?}", matches.len(), start.elapsed());
				}
				Err(err) => {
					error!("Try again, {err:?}");
				}
			}
		}
		let mut acc = 0.;
		for a in accuracies.iter() {
			acc += a;
		}
		acc /= 12.;
		info!("For SNR: {snr}%, Accuracy: {acc}, Raw: {accuracies:?}")
	}
}
