use std::{collections::HashMap, env, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use ratatui::style::{Color, Modifier, Style};
use serde::{de::Deserializer, Deserialize, Serialize};
use tracing::error;

use crate::{action::Action, app::Mode};

const CONFIG: &str = include_str!("../../../.config/config.json5");

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub uri: String,
}

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
    pub fn new() -> color_eyre::Result<Self, config::ConfigError> {
        let config_dir = get_config_dir();
        std::fs::create_dir_all(&config_dir).ok();
        let config_file = config_dir.join("config.json5");

        let mut builder = config::Config::builder();
        let default_config = Config::default();

        if config_file.exists() {
            builder = builder
                .add_source(config::File::from(config_file).format(config::FileFormat::Json5));
        }

        let cfg: Config = builder
            .build()
            .unwrap_or_default()
            .try_deserialize()
            .unwrap_or(default_config);
        Ok(cfg)
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        let config_dir = get_config_dir();
        std::fs::create_dir_all(&config_dir)?;
        let config_file = config_dir.join("config.json5");
        // Serialize only the AppConfig part
        let json = json5::to_string(&self.config)?;
        std::fs::write(config_file, json)?;
        Ok(())
    }
}

pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

pub fn get_config_dir() -> PathBuf {
    let directory = if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    };
    directory
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "kdheepak", env!("CARGO_PKG_NAME"))
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> color_eyre::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(KeyBindings(HashMap::new()))
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Styles(pub HashMap<Mode, HashMap<String, Style>>);

impl<'de> Deserialize<'de> for Styles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Styles(HashMap::new()))
    }
}
