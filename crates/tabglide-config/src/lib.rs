#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    time::Duration,
};
use tabglide_core::{ApplicationId, CoreConfig};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub activation_region_height: i32,
    pub focus_unfocused_window: bool,
    pub focus_return: bool,
    pub focus_return_delay_ms: u64,
    pub windows: WindowsConfig,
    pub diagnostics: DiagnosticsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            activation_region_height: 50,
            focus_unfocused_window: true,
            focus_return: true,
            focus_return_delay_ms: 700,
            windows: WindowsConfig::default(),
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowsConfig {
    pub allowed_applications: Vec<String>,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            allowed_applications: [
                "brave.exe",
                "chrome.exe",
                "chromium.exe",
                "explorer.exe",
                "firefox.exe",
                "msedge.exe",
                "opera.exe",
                "opera_gx.exe",
                "WindowsTerminal.exe",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiagnosticsConfig {
    pub logging: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("configuration I/O: {0}")]
    Io(#[from] io::Error),
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("cannot serialize defaults: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("invalid configuration: {0}")]
    Invalid(&'static str),
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(0..=10000).contains(&self.activation_region_height) {
            return Err(ConfigError::Invalid(
                "activation_region_height must be between 0 and 10000 pixels",
            ));
        }
        if self.focus_return_delay_ms > 60000 {
            return Err(ConfigError::Invalid(
                "focus_return_delay_ms must be between 0 and 60000",
            ));
        }
        if self
            .windows
            .allowed_applications
            .iter()
            .any(|name| name.trim().is_empty() || name.contains(['/', '\\', '\0']))
        {
            return Err(ConfigError::Invalid(
                "allowed_applications must contain executable basenames",
            ));
        }
        Ok(())
    }

    pub fn core_config(&self) -> CoreConfig {
        CoreConfig {
            activation_region_height: self.activation_region_height,
            focus_unfocused_window: self.focus_unfocused_window,
            focus_return: self.focus_return,
            focus_return_delay: Duration::from_millis(self.focus_return_delay_ms),
            allowed_applications: self
                .windows
                .allowed_applications
                .iter()
                .map(|name| ApplicationId::new(name))
                .collect(),
        }
    }
}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    Config::parse(&fs::read_to_string(path)?)
}

/// Creates a new file exclusively; never truncates a user's existing configuration.
pub fn load_or_create(path: &Path) -> Result<Config, ConfigError> {
    match load(path) {
        Err(ConfigError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let defaults = Config::default();
            let text = toml::to_string_pretty(&defaults)?;
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(mut file) => {
                    file.write_all(text.as_bytes())?;
                    file.sync_all()?;
                    Ok(defaults)
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => load(path),
                Err(error) => Err(error.into()),
            }
        }
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_and_partial_configuration() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.windows.allowed_applications.len(), 9);
        assert_eq!(config.focus_return_delay_ms, 700);
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(Config::parse(&text).unwrap().activation_region_height, 50);
        assert!(!Config::parse("focus_return = false").unwrap().focus_return);
        assert!(
            Config::parse("[windows]\nallowed_applications = []")
                .unwrap()
                .core_config()
                .allowed_applications
                .is_empty()
        );
    }

    #[test]
    fn rejects_invalid_values_and_typographical_errors() {
        for text in [
            "activation_region_height = -1",
            "focus_return_delay_ms = 60001",
            "focus_retrun = false",
            "[windows]\nallowed_applications = ['C:/app.exe']",
            "[diagnostics]\nloging = true",
            "focus_return = 'false'",
        ] {
            assert!(Config::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn existing_invalid_file_is_not_overwritten() {
        let directory =
            std::env::temp_dir().join(format!("tabglide-config-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.toml");
        fs::write(&path, "invalid TOML").unwrap();
        assert!(load_or_create(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid TOML");
        fs::remove_file(&path).unwrap();
        assert_eq!(load_or_create(&path).unwrap().focus_return_delay_ms, 700);
        let customized = "# preserve comments too\nfocus_return_delay_ms = 1234\n";
        fs::write(&path, customized).unwrap();
        assert_eq!(load_or_create(&path).unwrap().focus_return_delay_ms, 1234);
        assert_eq!(fs::read_to_string(&path).unwrap(), customized);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
