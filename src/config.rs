use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub const DEFAULT_STATION_URL: &str = "http://live-mp3-128.kexp.org/kexp128.mp3";
const APP_DIR: &str = "radio-slate";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub volume_percent: u8,
    pub default_station_url: String,
    pub tray_autoplay: bool,
    pub tray_icon: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            volume_percent: 70,
            default_station_url: DEFAULT_STATION_URL.to_string(),
            tray_autoplay: false,
            tray_icon: "audio-x-generic".to_string(),
        }
    }
}

impl AppConfig {
    fn sanitized(mut self) -> Self {
        self.volume_percent = self.volume_percent.min(100);
        if self.default_station_url.trim().is_empty() {
            self.default_station_url = DEFAULT_STATION_URL.to_string();
        }
        if self.tray_icon.trim().is_empty() {
            self.tray_icon = "audio-x-generic".to_string();
        }
        self
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    pub last_station_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileStore {
    config_path: PathBuf,
    state_path: PathBuf,
}

impl FileStore {
    pub fn from_env() -> Self {
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));

        let state_home = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("state"))
            })
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            config_path: config_home.join(APP_DIR).join("config.json"),
            state_path: state_home.join(APP_DIR).join("state.json"),
        }
    }

    #[cfg(test)]
    fn with_paths(config_path: PathBuf, state_path: PathBuf) -> Self {
        Self {
            config_path,
            state_path,
        }
    }

    pub fn load_config_or_default(&self) -> AppConfig {
        self.load_config().unwrap_or_else(|_| AppConfig::default())
    }

    pub fn load_state_or_default(&self) -> AppState {
        self.load_state().unwrap_or_default()
    }

    pub fn save_config(&self, config: &AppConfig) -> anyhow::Result<()> {
        self.write_json(&self.config_path, config)
    }

    pub fn save_state(&self, state: &AppState) -> anyhow::Result<()> {
        self.write_json(&self.state_path, state)
    }

    fn load_config(&self) -> anyhow::Result<AppConfig> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }

        let raw = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed reading {}", self.config_path.display()))?;
        let parsed = serde_json::from_str::<AppConfig>(&raw)
            .with_context(|| format!("failed parsing {}", self.config_path.display()))?;
        Ok(parsed.sanitized())
    }

    fn load_state(&self) -> anyhow::Result<AppState> {
        if !self.state_path.exists() {
            return Ok(AppState::default());
        }

        let raw = fs::read_to_string(&self.state_path)
            .with_context(|| format!("failed reading {}", self.state_path.display()))?;
        serde_json::from_str::<AppState>(&raw)
            .with_context(|| format!("failed parsing {}", self.state_path.display()))
    }

    fn write_json<T: Serialize>(&self, path: &Path, payload: &T) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed creating {}", parent.display()))?;
        }
        let body = serde_json::to_string_pretty(payload)?;
        fs::write(path, body).with_context(|| format!("failed writing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, AppState, FileStore};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_path(suffix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("radio-slate-tests-{suffix}-{nanos}"))
    }

    #[test]
    fn config_store_round_trips_config_and_state() {
        let root = unique_path("roundtrip");
        let store = FileStore::with_paths(root.join("config.json"), root.join("state.json"));

        let config = AppConfig {
            volume_percent: 42,
            default_station_url: "https://example.test/live".to_string(),
            tray_autoplay: true,
            tray_icon: "media-playback-start".to_string(),
        };
        let state = AppState {
            last_station_url: Some("https://example.test/last".to_string()),
        };

        store.save_config(&config).unwrap();
        store.save_state(&state).unwrap();

        assert_eq!(store.load_config().unwrap(), config);
        assert_eq!(store.load_state().unwrap(), state);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_store_uses_defaults_when_files_are_missing() {
        let root = unique_path("defaults");
        let store = FileStore::with_paths(root.join("config.json"), root.join("state.json"));

        let config = store.load_config_or_default();
        let state = store.load_state_or_default();

        assert_eq!(config, AppConfig::default());
        assert_eq!(state, AppState::default());
    }
}
