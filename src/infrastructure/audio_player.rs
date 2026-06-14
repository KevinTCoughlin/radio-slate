use std::{
    process::{Child, Command},
    sync::Mutex,
};

use crate::domain::{PlaybackError, Station};

pub trait AudioPlayback {
    fn play_station(&self, station: &Station) -> Result<(), PlaybackError>;
    fn stop_playback(&self) -> Result<(), PlaybackError>;
}

pub struct HttpAudioPlayer {
    active_child: Mutex<Option<Child>>,
}

impl HttpAudioPlayer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HttpAudioPlayer {
    fn default() -> Self {
        Self {
            active_child: Mutex::new(None),
        }
    }
}

impl AudioPlayback for HttpAudioPlayer {
    fn play_station(&self, station: &Station) -> Result<(), PlaybackError> {
        self.stop_playback()?;

        let mpv = Command::new("mpv")
            .args([
                "--no-video",
                "--really-quiet",
                "--no-terminal",
                "--volume=70",
                &station.url,
            ])
            .spawn();

        match mpv {
            Ok(child) => {
                self.active_child.lock().unwrap().replace(child);
                Ok(())
            }
            Err(_) => {
                let ffplay = Command::new("ffplay")
                    .args([
                        "-nodisp",
                        "-autoexit",
                        "-vn",
                        "-loglevel",
                        "quiet",
                        &station.url,
                    ])
                    .spawn();

                match ffplay {
                    Ok(child) => {
                        self.active_child.lock().unwrap().replace(child);
                        Ok(())
                    }
                    Err(error) => Err(PlaybackError::PlayerUnavailable(format!(
                        "failed to start audio player (mpv/ffplay): {error}"
                    ))),
                }
            }
        }
    }

    fn stop_playback(&self) -> Result<(), PlaybackError> {
        if let Some(mut child) = self.active_child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}
