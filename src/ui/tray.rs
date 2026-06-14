use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

use crate::application::PlaybackService;
use crate::domain::{PlaybackError, PlaybackState, Station, StationRepository};
use crate::infrastructure::{AudioPlayback, HttpAudioPlayer, InMemoryStationRepository};

const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";

pub fn tray_toggle_label(state: &PlaybackState) -> &'static str {
    match state {
        PlaybackState::Stopped => "Play KEXP",
        PlaybackState::Buffering(_) | PlaybackState::Playing(_) => "Stop KEXP",
    }
}

pub fn toggle_tray_playback<R: StationRepository, P: AudioPlayback>(
    service: &mut PlaybackService<R, P>,
) -> Result<PlaybackState, PlaybackError> {
    if service.selected_station().is_none() {
        service.select_default_station()?;
    }

    service.toggle_selected()
}

pub fn run_tray() -> anyhow::Result<()> {
    gtk::init().map_err(|_| anyhow::anyhow!("GTK initialization failed"))?;

    let stations = vec![Station::new("KEXP", DEFAULT_STATION_URL, "eclectic").unwrap()];
    let repository = InMemoryStationRepository::with_seed_stations(stations);
    let player = HttpAudioPlayer::new();
    let service = Arc::new(Mutex::new(PlaybackService::new(repository, player)));

    let mut indicator = AppIndicator::new("radio-slate", "audio-x-generic");
    indicator.set_status(AppIndicatorStatus::Active);

    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Play KEXP");
    let quit = gtk::MenuItem::with_label("Quit");

    let service_for_click = Arc::clone(&service);
    let toggle_label = toggle.clone();
    toggle.connect_activate(move |_| {
        let mut service = service_for_click.lock().unwrap();

        match toggle_tray_playback(&mut service) {
            Ok(state) => toggle_label.set_label(tray_toggle_label(&state)),
            Err(error) => eprintln!("playback toggle failed: {error}"),
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
