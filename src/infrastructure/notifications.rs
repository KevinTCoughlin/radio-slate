//! Desktop notification helpers for Fedora/Linux.
//!
//! Uses `notify-rust` which sends notifications via the
//! `org.freedesktop.Notifications` D-Bus service (libnotify-compatible).
//! All functions silently swallow errors so the rest of the application is
//! unaffected when no notification daemon is running (headless environments,
//! pure-SSH sessions, etc.).

const APP_NAME: &str = "radio-slate";
const APP_ICON: &str = "audio-x-generic";

/// Return the notification body for a "now playing" event.
///
/// Exposed separately so it can be tested without requiring a notification
/// daemon.
pub fn now_playing_body(station_name: &str) -> String {
    format!("Now playing: {station_name}")
}

/// Send a "Now playing" desktop notification for the given station.
///
/// Silently ignored when no notification daemon is available.
pub fn send_now_playing(station_name: &str) {
    let _ = notify_rust::Notification::new()
        .appname(APP_NAME)
        .summary(APP_NAME)
        .body(&now_playing_body(station_name))
        .icon(APP_ICON)
        .urgency(notify_rust::Urgency::Low)
        .show();
}

/// Send a "Stopped" desktop notification.
///
/// Silently ignored when no notification daemon is available.
pub fn send_stopped() {
    let _ = notify_rust::Notification::new()
        .appname(APP_NAME)
        .summary(APP_NAME)
        .body("Playback stopped")
        .icon(APP_ICON)
        .urgency(notify_rust::Urgency::Low)
        .show();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_playing_body_contains_station_name() {
        let body = now_playing_body("KEXP");
        assert!(
            body.contains("KEXP"),
            "body should mention the station name"
        );
    }

    #[test]
    fn now_playing_body_is_non_empty_for_empty_station() {
        // Even with an empty name the body must not be blank.
        let body = now_playing_body("");
        assert!(!body.is_empty());
    }

    #[test]
    fn now_playing_body_format_is_stable() {
        assert_eq!(now_playing_body("KEXP"), "Now playing: KEXP");
    }
}
