#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    TogglePlayback,
    NextStation,
}

pub fn shortcut_action_for_key(key: &str) -> Option<ShortcutAction> {
    let key = key.trim().to_ascii_lowercase();
    match key.as_str() {
        "space" | "p" | "k" | "xf86audioplay" | "xf86audiopause" | "xf86audiostop" => {
            Some(ShortcutAction::TogglePlayback)
        }
        "n" | "xf86audionext" => Some(ShortcutAction::NextStation),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortcutAction, shortcut_action_for_key};

    #[test]
    fn maps_toggle_shortcuts() {
        assert_eq!(
            shortcut_action_for_key("XF86AudioPlay"),
            Some(ShortcutAction::TogglePlayback)
        );
        assert_eq!(
            shortcut_action_for_key("space"),
            Some(ShortcutAction::TogglePlayback)
        );
    }

    #[test]
    fn maps_next_station_shortcut() {
        assert_eq!(
            shortcut_action_for_key("n"),
            Some(ShortcutAction::NextStation)
        );
    }
}
