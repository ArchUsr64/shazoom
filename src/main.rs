mod database;
mod encoder;

fn main() {
	let db_config = database::DatabaseConfig::default();
	let db_builder = database::DatabaseBuilder::default()
		.add_song("songs/tvari-hawaii-vacation-159069 (1).mp3.wav")
		.add_song("songs/Katy Perry - Harleys In Hawaii (Official) [sQEgklEwhSo].wav")
		.add_song("songs/Harleys In Hawaii (KANDY Remix) [JNfk_aQ-owo].wav")
		.add_song("songs/beyond-the-horizon-136339.mp3.wav");
	let db = db_builder.build(db_config);
	let mut matches = db.match_sample(encoder::Song::from_wav(&String::from(
		"songs/Harleys In Hawaii Sample.wav",
	)));
	matches.sort_by_key(|i| i.1);
	matches.reverse();
	dbg!(matches);
}
