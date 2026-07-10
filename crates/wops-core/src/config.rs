use std::{fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemePreference,
    pub window_width: f32,
    pub window_height: f32,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            window_width: 1280.0,
            window_height: 800.0,
            language: "en".to_owned(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("the operating system did not provide a configuration directory")]
    ConfigDirectoryUnavailable,
    #[error("could not access settings at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("settings at {path} contain invalid JSON: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from(Self::path()?)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(Self::path()?)
    }

    pub fn path() -> Result<PathBuf, ConfigError> {
        project_dirs()
            .map(|dirs| dirs.config_dir().join(SETTINGS_FILE))
            .ok_or(ConfigError::ConfigDirectoryUnavailable)
    }

    pub fn data_dir() -> Result<PathBuf, ConfigError> {
        project_dirs()
            .map(|dirs| dirs.data_dir().to_path_buf())
            .ok_or(ConfigError::ConfigDirectoryUnavailable)
    }

    fn load_from(path: PathBuf) -> Result<Self, ConfigError> {
        match fs::read_to_string(&path) {
            Ok(json) => {
                serde_json::from_str(&json).map_err(|source| ConfigError::Json { path, source })
            }
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io { path, source }),
        }
    }

    fn save_to(&self, path: PathBuf) -> Result<(), ConfigError> {
        let parent = path
            .parent()
            .ok_or(ConfigError::ConfigDirectoryUnavailable)?;
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.to_path_buf(),
            source,
        })?;

        let temporary_path = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self).map_err(|source| ConfigError::Json {
            path: path.clone(),
            source,
        })?;
        fs::write(&temporary_path, json).map_err(|source| ConfigError::Io {
            path: temporary_path.clone(),
            source,
        })?;
        fs::rename(&temporary_path, &path).map_err(|source| ConfigError::Io { path, source })
    }
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "wops", "wops")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("wops-{name}-{}", std::process::id()))
    }

    #[test]
    fn missing_settings_use_defaults() {
        let path = test_path("missing").join(SETTINGS_FILE);
        assert_eq!(Settings::load_from(path).unwrap(), Settings::default());
    }

    #[test]
    fn settings_round_trip_as_json() {
        let directory = test_path("round-trip");
        let path = directory.join(SETTINGS_FILE);
        let settings = Settings {
            theme: ThemePreference::Light,
            window_width: 1440.0,
            window_height: 900.0,
            language: "th".to_owned(),
        };

        settings.save_to(path.clone()).unwrap();
        assert_eq!(Settings::load_from(path).unwrap(), settings);
        fs::remove_dir_all(directory).unwrap();
    }
}
