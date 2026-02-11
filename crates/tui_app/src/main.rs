pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod config;
pub mod errors;
pub mod logging;
pub mod tui;

use clap::Parser;
use cli::Cli;
use color_eyre::eyre::Result;

use crate::{
    app::App,
    config::{get_config_dir, get_data_dir},
};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = tokio::fs::create_dir_all(get_data_dir()).await {
        eprintln!("Failed to create data directory: {}", e);
    }
    if let Err(e) = tokio::fs::create_dir_all(get_config_dir()).await {
        eprintln!("Failed to create config directory: {}", e);
    }

    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;
    Ok(())
}
