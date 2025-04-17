use anyhow::Result;
use models::{LyricLine, SongInfo, SongStatus};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod cmus;
mod lyrics;
mod models;

fn main() -> () {
    // Track current song path
    let current_song_path = Arc::new(Mutex::new(String::new()));
    let lyrics = Arc::new(Mutex::new(Result::<Option<Vec<LyricLine>>>::Ok(None)));

    // Main loop
    loop {
        // Get current song info from cmus
        let song_info = cmus::get_current_song();

        match song_info {
            Ok(Some(song)) => {
                if song.file_path != current_song_path.lock().unwrap().as_str() {
                    // Song changed
                    current_song_path.lock().unwrap().clear();
                    current_song_path.lock().unwrap().push_str(&song.file_path);
                    // Update lyrics
                    let new_lyrics = lyrics::get_lyrics(&song);
                    *lyrics.lock().unwrap() = new_lyrics;
                }

                if let Err(e) = update_lyrics(&lyrics.lock().unwrap(), &song) {
                    eprintln!("Error updating lyrics: {}", e);
                }
            }
            Ok(None) => {
                // No song playing
                cmus::render_no_song();
                thread::sleep(Duration::from_secs(1));
                continue;
            }
            Err(e) => {
                eprintln!("Error getting current song info: {}", e);
                cmus::render_no_song();
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

fn update_lyrics(lyrics_result: &Result<Option<Vec<LyricLine>>>, song: &SongInfo) -> Result<()> {
    // Get lyrics for the current song

    match lyrics_result {
        Ok(Some(lyrics_data)) => {
            if song.status == SongStatus::Paused {
                cmus::render_song_info(song);
                thread::sleep(Duration::from_secs(1));
                return Ok(());
            }
            // Find current lyric based on position
            let current_lyric = lyrics::get_lyrics_status(&lyrics_data, song.position);
            let tooltip = lyrics::format_lyrics_for_tooltip(&lyrics_data);

            cmus::render_lyrics(current_lyric.current_line, current_lyric.next_line, tooltip);
            // Calculate sleep duration based on next lyric timestamp
            if let Some(next_timestamp) = current_lyric.next_timestamp {
                let time_until_next = next_timestamp - song.position;
                if time_until_next > 0.0 {
                    // Sleep until the next lyric (with a small safety margin)
                    // Also ensure the sleep time doesn't exceed a maximum value (the user could switch songs in the meantime, if if the wait is too long it would bug)
                    thread::sleep(Duration::from_secs_f64(time_until_next.max(0.01).min(5.0)));
                } else {
                    // Fallback to shorter sleep if timing is off
                    thread::sleep(Duration::from_millis(100));
                }
            } else {
                // No next lyric, sleep for a longer time
                thread::sleep(Duration::from_secs(2));
            }
        }
        Ok(None) => {
            // No lyrics found
            cmus::render_song_info(song);
            thread::sleep(Duration::from_secs(2));
        }
        Err(e) => {
            eprintln!("Error getting lyrics: {}", e);
            // Error getting lyrics
            cmus::render_song_info(song);
            thread::sleep(Duration::from_secs(2));
        }
    }

    Ok(())
}
