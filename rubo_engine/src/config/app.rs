use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    name: String,
    config_path: PathBuf,
    profile: String,
    config_format: ConfigFileFormat,
    web: AppWebConfig,
    log: AppLogConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "rubo_engine".to_string(),
            config_path: PathBuf::from("config"),
            profile: String::new(),
            config_format: ConfigFileFormat::Json,
            web: AppWebConfig::default(),
            log: AppLogConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn set_profile(&mut self, profile: impl Into<String>) {
        self.profile = profile.into();
    }

    pub fn config_dir(&self) -> PathBuf {
        if self.profile.is_empty() {
            self.config_path.clone()
        } else {
            self.config_path.join(&self.profile)
        }
    }

    pub fn config_format(&self) -> ConfigFileFormat {
        self.config_format
    }

    pub fn web(&self) -> &AppWebConfig {
        &self.web
    }

    pub fn log(&self) -> &AppLogConfig {
        &self.log
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFileFormat {
    Json,
    Toml,
    Yaml,
}

impl ConfigFileFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppWebConfig {
    enabled: bool,
    host: String,
    port: u16,
}

impl Default for AppWebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 3888,
        }
    }
}

impl AppWebConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppLogConfig {
    enabled: bool,
    level: String,
}

impl Default for AppLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
        }
    }
}

impl AppLogConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn level(&self) -> &str {
        &self.level
    }
}
