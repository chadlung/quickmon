use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Persisted settings. Neither the network host nor the password lives here —
/// both are typed each run and live only in memory. Only UI convenience state
/// (e.g. the last directory used in Open/Save dialogs) is persisted to disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub last_dir: Option<PathBuf>,
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "quickmon")?;
        Some(dirs.config_dir().join("config.json"))
    }

    pub fn from_str_or_default(s: &str) -> Self {
        serde_json::from_str(s).unwrap_or_default()
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else { return Self::default() };
        match std::fs::read_to_string(&path) {
            Ok(s) => Self::from_str_or_default(&s),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else { return Ok(()) };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_config_has_no_host_or_password_field() {
        let cfg = Config { last_dir: None };
        let json = serde_json::to_string(&cfg).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("password"), "config must never carry a password: {json}");
        assert!(!lower.contains("host"), "config must never carry a host: {json}");
    }

    #[test]
    fn round_trips_last_dir() {
        let cfg = Config { last_dir: Some(std::path::PathBuf::from("/tmp/asm")) };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_dir, Some(std::path::PathBuf::from("/tmp/asm")));
    }

    #[test]
    fn default_has_no_last_dir() {
        let cfg = Config::default();
        assert!(cfg.last_dir.is_none());
    }

    #[test]
    fn load_returns_default_when_file_is_missing_or_corrupt() {
        assert_eq!(Config::from_str_or_default("not json at all").last_dir, None);
        assert_eq!(
            Config::from_str_or_default(r#"{"last_dir":"/tmp/asm"}"#).last_dir,
            Some(std::path::PathBuf::from("/tmp/asm"))
        );
    }
}
