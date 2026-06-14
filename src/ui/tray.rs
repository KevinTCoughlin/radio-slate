use std::{
    process::{Child, Command},
    sync::{Arc, Mutex},
};

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

use crate::config::AppConfig;

fn spawn_playback(station_url: &str, volume_percent: u8) -> std::io::Result<Child> {
    let volume = volume_percent.min(100).to_string();
    let mpv_volume = format!("--volume={volume}");
    let mpv = Command::new("mpv")
        .args([
            "--no-video",
            "--really-quiet",
            "--no-terminal",
            mpv_volume.as_str(),
            station_url,
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
                "-volume",
                &volume,
                station_url,
            ])
            .spawn(),
    }
}

fn stop_playback(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub fn run_tray(config: &AppConfig) -> anyhow::Result<()> {
    gtk::init().map_err(|_| anyhow::anyhow!("GTK initialization failed"))?;

    let active_child = Arc::new(Mutex::new(None::<Child>));
    let station_url = config.default_station_url.clone();
    let volume_percent = config.volume_percent;

    let mut indicator = AppIndicator::new("radio-slate", &config.tray_icon);
    indicator.set_status(AppIndicatorStatus::Active);

    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Play Radio");
    let quit = gtk::MenuItem::with_label("Quit");

    let active_child_for_click = Arc::clone(&active_child);
    let toggle_label = toggle.clone();
    let station_url_for_click = station_url.clone();
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
            toggle_label.set_label("Play Radio");
            return;
        }

        if let Ok(child) = spawn_playback(&station_url_for_click, volume_percent) {
            child_guard.replace(child);
            toggle_label.set_label("Stop Radio");
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

    if config.tray_autoplay {
        let mut child_guard = active_child.lock().unwrap();
        if let Ok(child) = spawn_playback(&station_url, volume_percent) {
            child_guard.replace(child);
            toggle.set_label("Stop Radio");
        } else {
            eprintln!("playback spawn failed");
        }
    }

    gtk::main();
    Ok(())
}
