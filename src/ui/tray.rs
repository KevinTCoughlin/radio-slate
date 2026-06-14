use std::{
    process::{Child, Command},
    sync::{Arc, Mutex},
};

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

use crate::domain::Station;
use crate::ui::metadata::{
    StationMetadata, format_metadata, parse_bitrate_from_url, parse_stream_title,
};
use crate::ui::shortcuts::{ShortcutAction, shortcut_action_for_key};

const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";

struct TrayState {
    child: Option<Child>,
    stations: Vec<Station>,
    station_index: usize,
}

fn build_stations() -> Vec<Station> {
    vec![Station::new("KEXP", DEFAULT_STATION_URL, "eclectic").unwrap()]
}

fn spawn_playback(url: &str) -> std::io::Result<Child> {
    let mpv = Command::new("mpv")
        .args([
            "--no-video",
            "--really-quiet",
            "--no-terminal",
            "--volume=70",
            url,
        ])
        .spawn();

    match mpv {
        Ok(child) => Ok(child),
        Err(_) => Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-vn", "-loglevel", "quiet", url])
            .spawn(),
    }
}

fn stop_playback(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn send_notification(summary: &str, body: &str) {
    let _ = Command::new("notify-send").args([summary, body]).spawn();
}

fn is_playing(state: &mut TrayState) -> bool {
    state
        .child
        .as_mut()
        .map(|child| child.try_wait().unwrap_or(None).is_none())
        .unwrap_or(false)
}

fn metadata_for_station(station: &Station) -> StationMetadata {
    let mut metadata = parse_stream_title(&station.name).unwrap_or_default();
    metadata.bitrate_kbps = parse_bitrate_from_url(&station.url);
    metadata
}

fn start_station_playback(state: &mut TrayState, toggle: &gtk::MenuItem, metadata: &gtk::MenuItem) {
    let Some(station) = state.stations.get(state.station_index) else {
        return;
    };

    match spawn_playback(&station.url) {
        Ok(child) => {
            state.child.replace(child);
            toggle.set_label(&format!("Pause {}", station.name));

            let station_metadata = metadata_for_station(station);
            let metadata_text = format_metadata(&station_metadata);
            metadata.set_label(&format!("Metadata: {metadata_text}"));
            send_notification(
                "radio-slate",
                &format!("Playing {} ({metadata_text})", station.name),
            );
        }
        Err(_) => {
            eprintln!("playback spawn failed");
            send_notification("radio-slate", "Playback failed to start");
        }
    }
}

fn toggle_playback(state: &mut TrayState, toggle: &gtk::MenuItem, metadata: &gtk::MenuItem) {
    if is_playing(state) {
        if let Some(child) = state.child.as_mut() {
            stop_playback(child);
        }
        state.child.take();
        if let Some(station) = state.stations.get(state.station_index) {
            toggle.set_label(&format!("Play {}", station.name));
        } else {
            toggle.set_label("Play");
        }
        metadata.set_label("Metadata: unavailable");
        send_notification("radio-slate", "Playback paused");
        return;
    }

    start_station_playback(state, toggle, metadata);
}

fn next_station(state: &mut TrayState, toggle: &gtk::MenuItem, metadata: &gtk::MenuItem) {
    if state.stations.is_empty() {
        return;
    }
    if let Some(child) = state.child.as_mut() {
        stop_playback(child);
    }
    state.child.take();
    state.station_index = (state.station_index + 1) % state.stations.len();
    start_station_playback(state, toggle, metadata);
}

pub fn run_tray() -> anyhow::Result<()> {
    gtk::init().map_err(|_| anyhow::anyhow!("GTK initialization failed"))?;

    let state = Arc::new(Mutex::new(TrayState {
        child: None,
        stations: build_stations(),
        station_index: 0,
    }));

    let mut indicator = AppIndicator::new("radio-slate", "audio-x-generic");
    indicator.set_status(AppIndicatorStatus::Active);

    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Play KEXP");
    let next = gtk::MenuItem::with_label("Next Station");
    let metadata = gtk::MenuItem::with_label("Metadata: unavailable");
    metadata.set_sensitive(false);
    let quit = gtk::MenuItem::with_label("Quit");

    let state_for_click = Arc::clone(&state);
    let toggle_for_click = toggle.clone();
    let metadata_for_click = metadata.clone();
    toggle.connect_activate(move |_| {
        if let Ok(mut state) = state_for_click.lock() {
            toggle_playback(&mut state, &toggle_for_click, &metadata_for_click);
        }
    });

    let state_for_next = Arc::clone(&state);
    let toggle_for_next = toggle.clone();
    let metadata_for_next = metadata.clone();
    next.connect_activate(move |_| {
        if let Ok(mut state) = state_for_next.lock() {
            next_station(&mut state, &toggle_for_next, &metadata_for_next);
        }
    });

    let state_for_key = Arc::clone(&state);
    let toggle_for_key = toggle.clone();
    let metadata_for_key = metadata.clone();
    menu.connect_key_press_event(move |_, event| {
        if let Some(key_name) = gtk::gdk::keyval_name(event.keyval()) {
            if let Some(action) = shortcut_action_for_key(key_name.as_str()) {
                if let Ok(mut state) = state_for_key.lock() {
                    match action {
                        ShortcutAction::TogglePlayback => {
                            toggle_playback(&mut state, &toggle_for_key, &metadata_for_key)
                        }
                        ShortcutAction::NextStation => {
                            next_station(&mut state, &toggle_for_key, &metadata_for_key)
                        }
                    }
                }
            }
        }
        gtk::Inhibit(false)
    });

    quit.connect_activate(|_| gtk::main_quit());

    menu.append(&toggle);
    menu.append(&next);
    menu.append(&metadata);
    menu.append(&gtk::SeparatorMenuItem::new());
    menu.append(&quit);
    menu.show_all();
    indicator.set_menu(&mut menu);

    gtk::main();
    Ok(())
}
