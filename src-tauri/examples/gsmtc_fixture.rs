//! Publishes only synthetic metadata/timing. No music service, network or audio.
//! Commands on stdin: play, pause, next, previous, seek <ms>, track <title>, quit.
use std::{
    io::{self, BufRead},
    sync::mpsc,
    time::{Duration, Instant},
};
use windows::{
    Foundation::{TimeSpan, TypedEventHandler},
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, Playback::MediaPlayer,
        SystemMediaTransportControls, SystemMediaTransportControlsButton,
        SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    },
    core::HSTRING,
};

fn main() -> windows::core::Result<()> {
    unsafe {
        windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        )?;
    }
    let player = MediaPlayer::new()?;
    player.CommandManager()?.SetIsEnabled(false)?;
    let controls = player.SystemMediaTransportControls()?;
    controls.SetIsEnabled(true)?;
    controls.SetIsPlayEnabled(true)?;
    controls.SetIsPauseEnabled(true)?;
    controls.SetIsNextEnabled(true)?;
    controls.SetIsPreviousEnabled(false)?;
    controls.SetPlaybackStatus(MediaPlaybackStatus::Playing)?;
    let updater = controls.DisplayUpdater()?;
    updater.SetType(MediaPlaybackType::Music)?;
    updater
        .MusicProperties()?
        .SetTitle(&HSTRING::from("Proyecto Reze — prueba propia"))?;
    updater
        .MusicProperties()?
        .SetArtist(&HSTRING::from("Proyecto Reze"))?;
    updater
        .MusicProperties()?
        .SetAlbumTitle(&HSTRING::from("Pruebas locales"))?;
    updater.Update()?;
    let (tx, rx) = mpsc::channel();
    let button_tx = tx.clone();
    let button_token = controls.ButtonPressed(&TypedEventHandler::<
        SystemMediaTransportControls,
        SystemMediaTransportControlsButtonPressedEventArgs,
    >::new(move |_, args| {
        if let Some(args) = args.as_ref() {
            let command = match args.Button()? {
                SystemMediaTransportControlsButton::Play => Some("play"),
                SystemMediaTransportControlsButton::Pause => Some("pause"),
                SystemMediaTransportControlsButton::Next => Some("next"),
                SystemMediaTransportControlsButton::Previous => Some("previous"),
                _ => None,
            };
            if let Some(command) = command {
                let _ = button_tx.send(command.to_owned());
            }
        }
        Ok(())
    }))?;
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
        let _ = tx.send("quit".into());
    });
    let mut position = 0i64;
    let mut track_index = 0u32;
    let mut playing = true;
    let mut anchor = Instant::now();
    println!("Fixture GSMTC: play | pause | seek <ms> | track <title> | quit");
    loop {
        if playing {
            position += anchor.elapsed().as_millis() as i64;
        }
        anchor = Instant::now();
        position = position.min(90_000);
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(command) => {
                if command == "quit" {
                    break;
                }
                if command == "play" {
                    playing = true;
                    controls.SetPlaybackStatus(MediaPlaybackStatus::Playing)?;
                }
                if command == "pause" {
                    playing = false;
                    controls.SetPlaybackStatus(MediaPlaybackStatus::Paused)?;
                }
                if let Some(value) = command
                    .strip_prefix("seek ")
                    .and_then(|s| s.parse::<i64>().ok())
                {
                    position = value.clamp(0, 90_000);
                    anchor = Instant::now();
                }
                if let Some(title) = command.strip_prefix("track ") {
                    updater.MusicProperties()?.SetTitle(&HSTRING::from(title))?;
                    updater.Update()?;
                    position = 0;
                    anchor = Instant::now();
                }
                if command == "next" || command == "previous" {
                    track_index = if command == "next" {
                        track_index.saturating_add(1)
                    } else {
                        track_index.saturating_sub(1)
                    };
                    let title = if track_index == 0 {
                        "Proyecto Reze — prueba propia".to_string()
                    } else {
                        format!("Proyecto Reze — prueba propia {}", track_index + 1)
                    };
                    updater.MusicProperties()?.SetTitle(&HSTRING::from(title))?;
                    updater.Update()?;
                    controls.SetIsPreviousEnabled(track_index > 0)?;
                    position = 0;
                    anchor = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        timeline.SetStartTime(TimeSpan { Duration: 0 })?;
        timeline.SetMinSeekTime(TimeSpan { Duration: 0 })?;
        timeline.SetEndTime(TimeSpan {
            Duration: 900_000_000,
        })?;
        timeline.SetMaxSeekTime(TimeSpan {
            Duration: 900_000_000,
        })?;
        timeline.SetPosition(TimeSpan {
            Duration: position * 10_000,
        })?;
        controls.UpdateTimelineProperties(&timeline)?;
    }
    controls.RemoveButtonPressed(button_token)?;
    controls.SetIsEnabled(false)?;
    drop(player);
    unsafe {
        windows::Win32::System::WinRT::RoUninitialize();
    }
    Ok(())
}
