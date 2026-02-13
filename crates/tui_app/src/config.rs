use std::{collections::HashMap, env, path::PathBuf};

use crossterm::event::KeyEvent;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use ratatui::style::Style;
use serde::{de::Deserializer, Deserialize, Serialize};
use tracing::error;

use crate::{action::Action, app::Mode};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub uri: String,
}

/// The persisted application configuration.
#[derive(Clone, Debug, Deserialize, Default, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
    #[serde(default)]
    pub connections: Vec<Connection>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: Styles,
}

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

impl Config {
    /// Load configuration with precedence:
    /// 1) Local `.mongo-tui.config.json`
    /// 2) OS config path `config.json`
    ///    If none exists, create a default in the OS config path.
    pub fn new() -> color_eyre::Result<Self, config::ConfigError> {
        let local_file = local_config_file();
        let os_file = os_config_file();
        let os_dir = get_config_dir();

        let mut builder = config::Config::builder();
        let default_config = Config::default();
        let mut found = false;

        if local_file.exists() {
            builder = builder.add_source(
                config::File::from(local_file.clone()).format(config::FileFormat::Json),
            );
            found = true;
        } else if os_file.exists() {
            builder = builder
                .add_source(config::File::from(os_file.clone()).format(config::FileFormat::Json));
            found = true;
        }

        let cfg: Config = builder
            .build()
            .unwrap_or_default()
            .try_deserialize()
            .unwrap_or_else(|_| default_config.clone());

        if !found {
            if let Err(e) = std::fs::create_dir_all(&os_dir) {
                error!(?e, "failed to create config directory");
            }
            if let Ok(json) = serde_json::to_string_pretty(&cfg.config) {
                if let Err(e) = std::fs::write(&os_file, json) {
                    error!(?e, "failed to write default config");
                }
            }
        }

        Ok(cfg)
    }

    /// Persist the configuration to the OS config path.
    pub fn save(&self) -> color_eyre::Result<()> {
        let config_dir = get_config_dir();
        std::fs::create_dir_all(&config_dir)?;
        let config_file = config_dir.join("config.json");
        let json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(config_file, json)?;
        Ok(())
    }
}

pub fn get_data_dir() -> PathBuf {
    if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    }
}

pub fn get_config_dir() -> PathBuf {
    if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    }
}

fn local_config_file() -> PathBuf {
    PathBuf::from(".mongo-tui.config.json")
}

fn os_config_file() -> PathBuf {
    get_config_dir().join("config.json")
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "kdheepak", env!("CARGO_PKG_NAME"))
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(_deserializer: D) -> color_eyre::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(KeyBindings(HashMap::new()))
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Styles(pub HashMap<Mode, HashMap<String, Style>>);

impl<'de> Deserialize<'de> for Styles {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Styles(HashMap::new()))
    }
}
