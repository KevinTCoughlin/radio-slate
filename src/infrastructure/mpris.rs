//! MPRIS2 D-Bus media player interface for Fedora/Linux desktop integration.
//!
//! Registers `org.mpris.MediaPlayer2.radio-slate` on the session D-Bus so that
//! GNOME Shell, the lock screen, and any MPRIS-aware client (e.g. playerctl,
//! KDE Connect) can read playback status and issue Play/Stop commands.  Media
//! keys on the keyboard are automatically routed through MPRIS by the desktop
//! environment, so no extra key-binding code is needed.
//!
//! # Fallback behaviour
//! [`spawn_mpris_service`] returns `None` when the session D-Bus is unavailable
//! (headless CI, SSH session without `DBUS_SESSION_BUS_ADDRESS`, etc.).  The
//! rest of the application continues running without MPRIS support.

use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread;

use zbus::{connection, interface, zvariant};

use crate::domain::PlaybackState;

const MPRIS_BUS_NAME: &str = "org.mpris.MediaPlayer2.radio-slate";
const MPRIS_OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";
const TRACK_ID_PATH: &str = "/org/radio_slate/track/current";

// ---------------------------------------------------------------------------
// Shared state (GTK thread writes, D-Bus thread reads)
// ---------------------------------------------------------------------------

/// Playback state shared between the GTK tray thread and the MPRIS D-Bus task.
#[derive(Debug, Clone)]
pub struct MprisSharedState {
    pub playback_status: String,
    pub station_name: String,
    pub station_url: String,
}

impl Default for MprisSharedState {
    fn default() -> Self {
        Self {
            playback_status: "Stopped".to_string(),
            station_name: String::new(),
            station_url: String::new(),
        }
    }
}

impl MprisSharedState {
    /// Build shared state from a domain [`PlaybackState`].
    ///
    /// Note: MPRIS has no "Buffering" status; it is reported as "Playing"
    /// because the stream has already been handed to the media player process.
    pub fn from_playback_state(
        state: PlaybackState,
        station_name: &str,
        station_url: &str,
    ) -> Self {
        let status = match state {
            PlaybackState::Playing(_) | PlaybackState::Buffering(_) => "Playing",
            PlaybackState::Stopped => "Stopped",
        };
        Self {
            playback_status: status.to_string(),
            station_name: station_name.to_string(),
            station_url: station_url.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Commands sent from the D-Bus interface back to the GTK tray
// ---------------------------------------------------------------------------

/// Commands that the MPRIS interface sends back to the GTK tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MprisCommand {
    Play,
    Pause,
    Stop,
    Toggle,
    Quit,
}

// ---------------------------------------------------------------------------
// Handle returned to the GTK tray for driving MPRIS state
// ---------------------------------------------------------------------------

/// Controller held by the GTK tray that drives the MPRIS D-Bus service.
///
/// Calling [`set_playing`](MprisHandle::set_playing) or
/// [`set_stopped`](MprisHandle::set_stopped) updates the state that the D-Bus
/// service advertises, and wakes the signal-emission task so that connected
/// clients receive `PropertiesChanged` immediately.
pub struct MprisHandle {
    pub(crate) state: Arc<Mutex<MprisSharedState>>,
    pub(crate) notify: Arc<tokio::sync::Notify>,
}

impl MprisHandle {
    /// Transition to the Playing state for the given station.
    pub fn set_playing(&self, station_name: &str, station_url: &str) {
        {
            let mut s = self.state.lock().unwrap();
            s.playback_status = "Playing".to_string();
            s.station_name = station_name.to_string();
            s.station_url = station_url.to_string();
        }
        self.notify.notify_one();
    }

    /// Transition to the Stopped state.
    pub fn set_stopped(&self) {
        {
            let mut s = self.state.lock().unwrap();
            s.playback_status = "Stopped".to_string();
        }
        self.notify.notify_one();
    }

    /// Returns the current MPRIS playback status string ("Playing" / "Stopped").
    pub fn playback_status(&self) -> String {
        self.state.lock().unwrap().playback_status.clone()
    }
}

// ---------------------------------------------------------------------------
// D-Bus interface implementations
// ---------------------------------------------------------------------------

struct MediaPlayer2Root;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MediaPlayer2Root {
    fn raise(&self) {}

    fn quit(&self) {
        gtk::main_quit();
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "radio-slate"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "radio-slate"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec!["http".into(), "https".into()]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec!["audio/mpeg".into(), "audio/ogg".into(), "audio/flac".into()]
    }
}

struct MediaPlayer2Player {
    state: Arc<Mutex<MprisSharedState>>,
    command_tx: SyncSender<MprisCommand>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MediaPlayer2Player {
    fn next(&self) {}
    fn previous(&self) {}

    fn pause(&self) {
        let _ = self.command_tx.send(MprisCommand::Pause);
    }

    fn play_pause(&self) {
        let _ = self.command_tx.send(MprisCommand::Toggle);
    }

    fn stop(&self) {
        let _ = self.command_tx.send(MprisCommand::Stop);
    }

    fn play(&self) {
        let _ = self.command_tx.send(MprisCommand::Play);
    }

    fn seek(&self, _offset: i64) {}

    fn set_position(&self, _track_id: zvariant::ObjectPath<'_>, _position: i64) {}

    fn open_uri(&self, _uri: &str) {}

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.state.lock().unwrap().playback_status.clone()
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        "None".to_string()
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, zvariant::OwnedValue> {
        let s = self.state.lock().unwrap();
        build_metadata(&s.station_name, &s.station_url)
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        0.7
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        0
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Metadata helper
// ---------------------------------------------------------------------------

/// Build the MPRIS `Metadata` dict (`a{sv}`) for the current station.
///
/// `mpris:trackid` is always present (required by spec); `xesam:title` and
/// `xesam:url` are included when non-empty.
pub fn build_metadata(
    station_name: &str,
    station_url: &str,
) -> HashMap<String, zvariant::OwnedValue> {
    let mut map: HashMap<String, zvariant::OwnedValue> = HashMap::new();

    // mpris:trackid is mandatory (must be an object path, not a plain string).
    if let Ok(path) = zvariant::ObjectPath::try_from(TRACK_ID_PATH) {
        let static_path: zvariant::ObjectPath<'static> =
            zvariant::ObjectPath::try_from(TRACK_ID_PATH).unwrap_or(path);
        let val: zvariant::Value<'static> = zvariant::Value::ObjectPath(static_path);
        if let Ok(owned) = zvariant::OwnedValue::try_from(val) {
            map.insert("mpris:trackid".to_string(), owned);
        }
    }

    if !station_name.is_empty() {
        let val: zvariant::Value<'static> = zvariant::Value::Str(station_name.to_string().into());
        if let Ok(owned) = zvariant::OwnedValue::try_from(val) {
            map.insert("xesam:title".to_string(), owned);
        }
    }

    if !station_url.is_empty() {
        let val: zvariant::Value<'static> = zvariant::Value::Str(station_url.to_string().into());
        if let Ok(owned) = zvariant::OwnedValue::try_from(val) {
            map.insert("xesam:url".to_string(), owned);
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Service launcher
// ---------------------------------------------------------------------------

/// Spawn the MPRIS D-Bus service in a background thread.
///
/// Returns a [`MprisHandle`] the tray can use to push state updates and a
/// [`std::sync::mpsc::Receiver`] for commands that arrive from MPRIS clients
/// (Play, Stop, Toggle, Quit).
///
/// Returns `None` when the session D-Bus is unavailable (e.g. headless
/// environment), letting the caller continue without MPRIS support.
pub fn spawn_mpris_service(command_tx: SyncSender<MprisCommand>) -> Option<MprisHandle> {
    let state = Arc::new(Mutex::new(MprisSharedState::default()));
    let notify = Arc::new(tokio::sync::Notify::new());

    let state_svc = Arc::clone(&state);
    let notify_svc = Arc::clone(&notify);
    let command_tx_svc = command_tx;

    // Use a one-shot channel to report whether the D-Bus connection succeeded.
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<bool>(1);

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        let rt = match rt {
            Ok(r) => r,
            Err(err) => {
                eprintln!("radio-slate: tokio runtime build failed: {err}");
                let _ = ready_tx.send(false);
                return;
            }
        };

        rt.block_on(async move {
            let player = MediaPlayer2Player {
                state: Arc::clone(&state_svc),
                command_tx: command_tx_svc,
            };

            let conn_result = connection::Builder::session()
                .and_then(|b| b.name(MPRIS_BUS_NAME))
                .and_then(|b| b.serve_at(MPRIS_OBJECT_PATH, MediaPlayer2Root))
                .and_then(|b| b.serve_at(MPRIS_OBJECT_PATH, player));

            let conn = match conn_result {
                Ok(b) => match b.build().await {
                    Ok(c) => c,
                    Err(err) => {
                        eprintln!("radio-slate: MPRIS D-Bus connection failed: {err}");
                        let _ = ready_tx.send(false);
                        return;
                    }
                },
                Err(err) => {
                    eprintln!("radio-slate: MPRIS setup failed: {err}");
                    let _ = ready_tx.send(false);
                    return;
                }
            };

            // Signal that we connected successfully.
            let _ = ready_tx.send(true);

            // Watch for state changes from the GTK thread and emit
            // PropertiesChanged so GNOME Shell and other clients update.
            let notify_task = Arc::clone(&notify_svc);
            tokio::spawn(async move {
                loop {
                    notify_task.notified().await;
                    if let Ok(iface_ref) = conn
                        .object_server()
                        .interface::<_, MediaPlayer2Player>(MPRIS_OBJECT_PATH)
                        .await
                    {
                        let emitter = iface_ref.signal_emitter();
                        let guard = iface_ref.get().await;
                        let _ = MediaPlayer2Player::playback_status_changed(&guard, emitter).await;
                        let _ = MediaPlayer2Player::metadata_changed(&guard, emitter).await;
                    }
                }
            });

            // Keep the thread (and the D-Bus connection) alive indefinitely.
            std::future::pending::<()>().await;
        });
    });

    // Wait up to one second for the connection attempt to resolve.
    match ready_rx.recv_timeout(std::time::Duration::from_secs(1)) {
        Ok(true) => Some(MprisHandle { state, notify }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handle() -> MprisHandle {
        MprisHandle {
            state: Arc::new(Mutex::new(MprisSharedState::default())),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    #[test]
    fn handle_starts_stopped() {
        let h = make_handle();
        assert_eq!(h.playback_status(), "Stopped");
    }

    #[test]
    fn handle_transitions_to_playing() {
        let h = make_handle();
        h.set_playing("KEXP", "http://live-mp3-128.kexp.org/kexp128.mp3");
        assert_eq!(h.playback_status(), "Playing");
        let s = h.state.lock().unwrap();
        assert_eq!(s.station_name, "KEXP");
        assert_eq!(s.station_url, "http://live-mp3-128.kexp.org/kexp128.mp3");
    }

    #[test]
    fn handle_transitions_back_to_stopped() {
        let h = make_handle();
        h.set_playing("KEXP", "http://live-mp3-128.kexp.org/kexp128.mp3");
        h.set_stopped();
        assert_eq!(h.playback_status(), "Stopped");
    }

    #[test]
    fn shared_state_maps_playing_domain_state() {
        let s = MprisSharedState::from_playback_state(
            PlaybackState::Playing(crate::domain::StationSelection::new(
                crate::domain::StationId::new("kexp"),
            )),
            "KEXP",
            "http://kexp.test/stream",
        );
        assert_eq!(s.playback_status, "Playing");
        assert_eq!(s.station_name, "KEXP");
    }

    #[test]
    fn shared_state_maps_stopped_domain_state() {
        let s = MprisSharedState::from_playback_state(PlaybackState::Stopped, "", "");
        assert_eq!(s.playback_status, "Stopped");
    }

    #[test]
    fn shared_state_maps_buffering_to_playing() {
        let s = MprisSharedState::from_playback_state(
            PlaybackState::Buffering(crate::domain::StationSelection::new(
                crate::domain::StationId::new("kexp"),
            )),
            "KEXP",
            "http://kexp.test/stream",
        );
        assert_eq!(s.playback_status, "Playing");
    }

    #[test]
    fn metadata_always_contains_trackid() {
        let m = build_metadata("KEXP", "http://kexp.test/stream");
        assert!(
            m.contains_key("mpris:trackid"),
            "mpris:trackid is required by spec"
        );
    }

    #[test]
    fn metadata_includes_title_when_present() {
        let m = build_metadata("KEXP", "");
        assert!(m.contains_key("xesam:title"));
    }

    #[test]
    fn metadata_excludes_title_when_empty() {
        let m = build_metadata("", "");
        assert!(!m.contains_key("xesam:title"));
    }

    #[test]
    fn metadata_includes_url_when_present() {
        let m = build_metadata("", "http://kexp.test/stream");
        assert!(m.contains_key("xesam:url"));
    }
}
