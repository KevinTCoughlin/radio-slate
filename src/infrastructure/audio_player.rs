use std::process::Command;

use crate::domain::{PlaybackState, Station};

pub trait AudioPlayback {
    fn play_station(&self, station: &Station) -> anyhow::Result<()>;
    fn status_label(&self, state: PlaybackState) -> &'static str;
}

#[derive(Default)]
pub struct HttpAudioPlayer;

impl HttpAudioPlayer {
    pub fn new() -> Self {
        Self
    }
}

impl AudioPlayback for HttpAudioPlayer {
    fn play_station(&self, station: &Station) -> anyhow::Result<()> {
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
            Ok(_) => Ok(()),
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
                    Ok(_) => Ok(()),
                    Err(error) => Err(anyhow::anyhow!(
                        "failed to start audio player (mpv/ffplay): {error}"
                    )),
                }
            }
        }
    }

    fn status_label(&self, state: PlaybackState) -> &'static str {
        match state {
            PlaybackState::Stopped => "stopped",
            PlaybackState::Playing => "playing",
            PlaybackState::Buffering => "buffering",
        }
    }
}
