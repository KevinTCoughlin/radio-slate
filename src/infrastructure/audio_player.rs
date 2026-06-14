use std::process::Command;

use crate::domain::{PlaybackState, Station};

pub trait AudioPlayback {
    fn play_station(&self, station: &Station) -> anyhow::Result<()>;
    fn status_label(&self, state: PlaybackState) -> &'static str;
}

pub struct HttpAudioPlayer {
    volume_percent: u8,
}

impl HttpAudioPlayer {
    pub fn new(volume_percent: u8) -> Self {
        Self {
            volume_percent: volume_percent.min(100),
        }
    }
}

impl AudioPlayback for HttpAudioPlayer {
    fn play_station(&self, station: &Station) -> anyhow::Result<()> {
        let volume = self.volume_percent.to_string();
        let mpv_volume = format!("--volume={volume}");
        let mpv = Command::new("mpv")
            .args([
                "--no-video",
                "--really-quiet",
                "--no-terminal",
                mpv_volume.as_str(),
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
                        "-volume",
                        &volume,
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
