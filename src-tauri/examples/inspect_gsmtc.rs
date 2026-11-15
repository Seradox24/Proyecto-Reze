//! Read-only diagnostic: uses the same adapter as Proyecto Reze, never a music service API.
#[path = "../src/media.rs"]
mod media;
use proyecto_reze_core::model::MediaSource;
fn main() -> windows::core::Result<()> {
    unsafe {
        windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        )?;
    }
    let (tx, _rx) = std::sync::mpsc::sync_channel(1);
    let mut source = media::Gsmtc::new(tx)?;
    let listing = source.sample(None).expect("GSMTC listing failed");
    println!("Sesiones: {}", listing.sessions.len());
    for session in listing.sessions {
        let first = source.sample(Some(&session.id)).expect("GSMTC read failed");
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let next = source.sample(Some(&session.id)).expect("GSMTC read failed");
        println!(
            "{}",
            serde_json::json!({"application": session.name, "track": next.track, "timeline": next.timeline, "previous_position_ms":first.timeline.position_ms})
        );
    }
    Ok(())
}
