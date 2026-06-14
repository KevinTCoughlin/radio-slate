use std::{
    io::ErrorKind,
    process::{Child, Command},
    sync::{Arc, Mutex},
};

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";

#[derive(Debug, Clone, Copy)]
struct PlayerLauncher {
    name: &'static str,
    binary: &'static str,
    args: &'static [&'static str],
}

const MPV_LAUNCHER: PlayerLauncher = PlayerLauncher {
    name: "mpv",
    binary: "mpv",
    args: &[
        "--no-video",
        "--really-quiet",
        "--no-terminal",
        "--volume=70",
    ],
};

const FFPLAY_LAUNCHER: PlayerLauncher = PlayerLauncher {
    name: "ffplay",
    binary: "ffplay",
    args: &["-nodisp", "-autoexit", "-vn", "-loglevel", "quiet"],
};

#[derive(Debug)]
struct PlaybackProcess {
    child: Child,
    backend: &'static str,
}

fn status_text(is_playing: bool, is_buffering: bool) -> &'static str {
    if is_buffering {
        "Status: buffering"
    } else if is_playing {
        "Status: playing"
    } else {
        "Status: stopped"
    }
}

fn spawn_with_fallback(
    stream_url: &str,
    launchers: &[PlayerLauncher],
) -> anyhow::Result<PlaybackProcess> {
    let mut startup_errors = Vec::new();

    for launcher in launchers {
        match Command::new(launcher.binary)
            .args(launcher.args)
            .arg(stream_url)
            .spawn()
        {
            Ok(child) => {
                return Ok(PlaybackProcess {
                    child,
                    backend: launcher.name,
                });
            }
            Err(error) => startup_errors.push(format!("{}: {error}", launcher.name)),
        }
    }

    Err(anyhow::anyhow!(
        "failed to start playback process with any configured player ({}). Ensure mpv or ffplay is installed and available on PATH.",
        startup_errors.join("; ")
    ))
}

fn spawn_playback() -> anyhow::Result<PlaybackProcess> {
    spawn_with_fallback(DEFAULT_STATION_URL, &[MPV_LAUNCHER, FFPLAY_LAUNCHER])
}

fn playback_is_running(playback: &mut PlaybackProcess) -> anyhow::Result<bool> {
    playback
        .child
        .try_wait()
        .map(|status| status.is_none())
        .map_err(|error| anyhow::anyhow!("failed to inspect playback process state: {error}"))
}

fn stop_playback(playback: &mut PlaybackProcess) -> anyhow::Result<()> {
    if playback
        .child
        .try_wait()
        .map_err(|error| anyhow::anyhow!("failed to inspect playback process state: {error}"))?
        .is_some()
    {
        return Ok(());
    }

    if let Err(error) = playback.child.kill()
        && !matches!(error.kind(), ErrorKind::InvalidInput | ErrorKind::NotFound)
    {
        return Err(anyhow::anyhow!("failed to stop playback process: {error}"));
    }

    playback
        .child
        .wait()
        .map_err(|error| anyhow::anyhow!("failed to reap playback process: {error}"))?;
    Ok(())
}

pub fn run_tray() -> anyhow::Result<()> {
    gtk::init().map_err(|_| anyhow::anyhow!("GTK initialization failed"))?;

    let active_child = Arc::new(Mutex::new(None::<PlaybackProcess>));

    let mut indicator = AppIndicator::new("radio-slate", "audio-x-generic");

    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Play KEXP");
    let status = gtk::MenuItem::with_label(status_text(false, false));
    status.set_sensitive(false);
    let quit = gtk::MenuItem::with_label("Quit");

    let active_child_for_click = Arc::clone(&active_child);
    let toggle_label = toggle.clone();
    let status_label = status.clone();
    toggle.connect_activate(move |_| {
        let mut child_guard = active_child_for_click.lock().unwrap();

        let was_running = match child_guard.as_mut() {
            Some(playback) => match playback_is_running(playback) {
                Ok(is_running) => is_running,
                Err(error) => {
                    eprintln!("playback status check failed: {error}");
                    false
                }
            },
            None => false,
        };

        if was_running {
            status_label.set_label(status_text(false, true));
            if let Some(child) = child_guard.as_mut()
                && let Err(error) = stop_playback(child)
            {
                eprintln!("playback stop failed: {error}");
            }
            child_guard.take();
            toggle_label.set_label("Play KEXP");
            status_label.set_label(status_text(false, false));
            return;
        }

        child_guard.take();
        status_label.set_label(status_text(false, true));
        match spawn_playback() {
            Ok(child) => {
                eprintln!("playback started with {}", child.backend);
                child_guard.replace(child);
                toggle_label.set_label("Stop KEXP");
                status_label.set_label(status_text(true, false));
            }
            Err(error) => {
                eprintln!("playback spawn failed: {error}");
                toggle_label.set_label("Play KEXP");
                status_label.set_label(status_text(false, false));
            }
        }
    });

    quit.connect_activate(|_| gtk::main_quit());

    menu.append(&toggle);
    menu.append(&status);
    menu.append(&gtk::SeparatorMenuItem::new());
    menu.append(&quit);
    menu.show_all();
    indicator.set_menu(&mut menu);
    indicator.set_status(AppIndicatorStatus::Active);

    gtk::main();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PlayerLauncher, playback_is_running, spawn_with_fallback, stop_playback};
    use std::{thread, time::Duration};

    const MISSING_LAUNCHER: PlayerLauncher = PlayerLauncher {
        name: "missing",
        binary: "radio-slate-definitely-missing",
        args: &[],
    };

    const SHELL_SLEEP_LAUNCHER: PlayerLauncher = PlayerLauncher {
        name: "shell",
        binary: "sh",
        args: &["-c", "sleep 30"],
    };

    const SHELL_EXIT_LAUNCHER: PlayerLauncher = PlayerLauncher {
        name: "shell",
        binary: "sh",
        args: &["-c", "exit 0"],
    };

    #[test]
    fn fallback_uses_secondary_launcher_when_primary_fails() {
        let mut playback = spawn_with_fallback(
            "https://example.test/stream",
            &[MISSING_LAUNCHER, SHELL_SLEEP_LAUNCHER],
        )
        .expect("fallback launcher should start");

        assert_eq!(playback.backend, "shell");
        assert!(playback_is_running(&mut playback).unwrap());
        stop_playback(&mut playback).unwrap();
    }

    #[test]
    fn startup_failure_reports_all_attempts() {
        let error = spawn_with_fallback(
            "https://example.test/stream",
            &[MISSING_LAUNCHER, MISSING_LAUNCHER],
        )
        .expect_err("all launchers should fail");

        let text = error.to_string();
        assert!(text.contains("failed to start playback process"));
        assert!(text.contains("missing"));
    }

    #[test]
    fn stopping_already_exited_process_is_clean() {
        let mut playback =
            spawn_with_fallback("https://example.test/stream", &[SHELL_EXIT_LAUNCHER])
                .expect("launcher should start");
        thread::sleep(Duration::from_millis(50));

        stop_playback(&mut playback).expect("stop should succeed for an already-exited process");
    }
}
