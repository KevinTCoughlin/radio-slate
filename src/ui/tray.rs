use std::{
    process::{Child, Command},
    sync::{Arc, Mutex},
};

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";

fn spawn_playback() -> std::io::Result<Child> {
    let mpv = Command::new("mpv")
        .args([
            "--no-video",
            "--really-quiet",
            "--no-terminal",
            "--volume=70",
            DEFAULT_STATION_URL,
        ])
        .spawn();

    match mpv {
        Ok(child) => Ok(child),
        Err(_) => Command::new("ffplay")
            .args([
                "-nodisp",
                "-autoexit",
                "-vn",
                "-loglevel",
                "quiet",
                DEFAULT_STATION_URL,
            ])
            .spawn(),
    }
}

fn stop_playback(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub fn run_tray() -> anyhow::Result<()> {
    gtk::init().map_err(|_| anyhow::anyhow!("GTK initialization failed"))?;

    let active_child = Arc::new(Mutex::new(None::<Child>));

    let mut indicator = AppIndicator::new("radio-slate", "audio-x-generic");
    indicator.set_status(AppIndicatorStatus::Active);

    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Play KEXP");
    let quit = gtk::MenuItem::with_label("Quit");

    let active_child_for_click = Arc::clone(&active_child);
    let toggle_label = toggle.clone();
    toggle.connect_activate(move |_| {
        let mut child_guard = active_child_for_click.lock().unwrap();

        let was_running = child_guard
            .as_mut()
            .map(|child| child.try_wait().unwrap_or(None).is_none())
            .unwrap_or(false);

        if was_running {
            if let Some(child) = child_guard.as_mut() {
                stop_playback(child);
            }
            child_guard.take();
            toggle_label.set_label("Play KEXP");
            return;
        }

        if let Ok(child) = spawn_playback() {
            child_guard.replace(child);
            toggle_label.set_label("Stop KEXP");
        } else {
            eprintln!("playback spawn failed");
        }
    });

    quit.connect_activate(|_| gtk::main_quit());

    menu.append(&toggle);
    menu.append(&gtk::SeparatorMenuItem::new());
    menu.append(&quit);
    menu.show_all();
    indicator.set_menu(&mut menu);

    gtk::main();
    Ok(())
}
