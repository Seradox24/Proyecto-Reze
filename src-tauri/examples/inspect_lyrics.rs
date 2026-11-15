//! Explicit diagnostic only. Prints metadata and result summaries, never lyrics.
#[path = "../src/media.rs"]
#[allow(dead_code)]
mod media;
use proyecto_reze_core::{
    lyrics::{Lrclib, LyricsProvider, automatic_match},
    matching::identity,
    model::{MediaSource, Track},
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        )?;
    }
    let track = if std::env::args().any(|arg| arg == "--heman-regression") {
        Track {
            title: "HE-MAN Y Los Amos Del Universo Intro Cover!".into(),
            artist: "The Covers Duo".into(),
            ..Default::default()
        }
    } else if std::env::args().any(|arg| arg == "--cover-regression") {
        Track {
            title: "Black Clover Opening 5 『Miyuna - Gamushara』Cover [ES]".into(),
            artist: "The Covers Duo".into(),
            duration_ms: Some(216_121),
            ..Default::default()
        }
    } else {
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        let mut source = media::Gsmtc::new(tx)?;
        let listing = source.sample(None)?;
        let session = listing
            .sessions
            .iter()
            .find(|s| s.name.to_lowercase().contains("brave"))
            .ok_or("No Brave GSMTC session")?;
        source.sample(Some(&session.id))?.track.ok_or("No track")?
    };
    let derived = identity(&track);
    let results = Lrclib::new()?.search(&track).await?;
    println!(
        "{}",
        serde_json::json!({"query_title":derived.title,"query_artist":derived.artist,"count":results.len(),"automatic":automatic_match(&track,&results),"candidates":results.iter().map(|c| serde_json::json!({"id":c.id,"title":c.track_name,"artist":c.artist_name,"duration":c.duration,"synced":c.synced_lyrics.as_ref().is_some_and(|s| !s.is_empty()),"note":c.match_note})).collect::<Vec<_>>()})
    );
    Ok(())
}
