#![feature(stmt_expr_attributes)]

use clap::Parser;
use log::{debug, error, info};

mod database;
mod encoder;

use crate::database::SongId;
use crate::encoder::{Freq, TimeStamp};

#[derive(Parser)]
pub struct Args {
	#[arg(short, long, default_value_t = 50)]
	pub ms_timeslice_size: u64,
	#[arg(short, long, default_value_t = 8)]
	pub freq_per_slice: usize,
	#[arg(short, long, default_value_t = 120)]
	pub size_bucket: Freq,
	#[arg(short, long, default_value_t = 32)]
	pub count_bucket: usize,
	#[arg(short, long, default_value_t = 5)]
	pub width_target_zone: TimeStamp,
	#[arg(short, long, default_value_t = 600)]
	pub target_zone_height: Freq,
	#[arg(long, default_value_t = 0b1)]
	pub fuzz_factor: Freq,
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
	let start = std::time::Instant::now();
	let samples = [
		"samples/Attention.ogg.wav",
		"samples/CheapThrills.ogg.wav",
		"samples/Closer.ogg.wav",
		"samples/Faded.ogg.wav",
		"samples/HymmForTheWeekend.ogg.wav",
		"samples/NewRules.ogg.wav",
		"samples/NotAfraid.ogg.wav",
		"samples/Starboy.ogg.wav",
		"samples/TakiTaki.ogg.wav",
		"samples/Timber.ogg.wav",
	];

	let mut cummulative_noise_gm = 1f32;
	let mut cummulative_noise_am = 0f32;
	for (sample_id, (path, sample)) in samples
		.iter()
		.map(|i| (i, encoder::Song::from_wav(&i.to_string())))
		.enumerate()
	{
		let sample_id = sample_id as SongId;
		let mut matches = db.match_sample(sample);
		matches.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
		matches.reverse();
		if matches[0].id == sample_id {
			info!("Successful Match");
		} else {
			error!("Match failed");
		}
		info!("For {}", path);
		info!("{:?}", &matches[..2.min(matches.len())]);
		let match_score = matches[0].score;
		let noise = matches.iter().map(|i| i.score).sum::<f32>() / match_score as f32;
		cummulative_noise_gm *= noise;
		cummulative_noise_am += noise;
		info!("Score: {match_score}, Noise: {}", noise - 1.);
	}

	cummulative_noise_gm = cummulative_noise_gm.powf((samples.len() as f32).recip());
	cummulative_noise_am = cummulative_noise_am / samples.len() as f32;
	info!("Cum Noise GM: {}", cummulative_noise_gm);
	info!("Cum Noise AM: {}", cummulative_noise_am);
	info!("Matching Took {:#?}", start.elapsed());
}
