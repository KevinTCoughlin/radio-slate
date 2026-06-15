use std::{
    process::{Child, Command},
    sync::{Arc, Mutex},
    time::Duration,
};

use gtk::glib;
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

use crate::infrastructure::{
    send_now_playing, send_stopped, spawn_mpris_service, MprisCommand,
};

const DEFAULT_STATION_NAME: &str = "KEXP";
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

    // --- MPRIS D-Bus service -------------------------------------------
    // A bounded sync channel carries commands from MPRIS clients (e.g.
    // GNOME Shell media keys) back to the GTK main loop.
    let (mpris_cmd_tx, mpris_cmd_rx) = std::sync::mpsc::sync_channel::<MprisCommand>(32);
    let mpris_handle = spawn_mpris_service(mpris_cmd_tx);

    // --- Tray indicator ------------------------------------------------
    let mut indicator = AppIndicator::new("radio-slate", "audio-x-generic");
    indicator.set_status(AppIndicatorStatus::Active);

    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Play KEXP");
    let quit = gtk::MenuItem::with_label("Quit");

    let active_child_for_click = Arc::clone(&active_child);
    let toggle_label = toggle.clone();
    let mpris_handle_arc = mpris_handle.map(Arc::new);
    let mpris_arc_click = mpris_handle_arc.clone();

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

            if let Some(ref h) = mpris_arc_click {
                h.set_stopped();
            }
            send_stopped();
            return;
        }

        if let Ok(child) = spawn_playback() {
            child_guard.replace(child);
            toggle_label.set_label("Stop KEXP");

            if let Some(ref h) = mpris_arc_click {
                h.set_playing(DEFAULT_STATION_NAME, DEFAULT_STATION_URL);
            }
            send_now_playing(DEFAULT_STATION_NAME);
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

    // --- Route MPRIS commands back into the GTK event loop ------------
    // Poll the channel at 100 ms intervals; each tick drains all pending
    // commands so latency from media keys is imperceptible.
    let toggle_from_mpris = toggle.clone();
    let active_child_mpris = Arc::clone(&active_child);
    let mpris_handle_poll = mpris_handle_arc.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(cmd) = mpris_cmd_rx.try_recv() {
            match cmd {
                MprisCommand::Play => {
                    let mut guard = active_child_mpris.lock().unwrap();
                    let running = guard
                        .as_mut()
                        .map(|c| c.try_wait().unwrap_or(None).is_none())
                        .unwrap_or(false);
                    if !running && let Ok(child) = spawn_playback() {
                        guard.replace(child);
                        toggle_from_mpris.set_label("Stop KEXP");
                        if let Some(ref h) = mpris_handle_poll {
                            h.set_playing(DEFAULT_STATION_NAME, DEFAULT_STATION_URL);
                        }
                        send_now_playing(DEFAULT_STATION_NAME);
                    }
                }
                MprisCommand::Stop | MprisCommand::Pause => {
                    let mut guard = active_child_mpris.lock().unwrap();
                    if let Some(child) = guard.as_mut() {
                        stop_playback(child);
                    }
                    guard.take();
                    toggle_from_mpris.set_label("Play KEXP");
                    if let Some(ref h) = mpris_handle_poll {
                        h.set_stopped();
                    }
                    send_stopped();
                }
                MprisCommand::Toggle => {
                    // Emit a synthetic click on the toggle menu item.
                    toggle_from_mpris.activate();
                }
                MprisCommand::Quit => gtk::main_quit(),
            }
        }
        glib::ControlFlow::Continue
    });

    gtk::main();
    Ok(())
}
