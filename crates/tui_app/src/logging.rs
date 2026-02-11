use crate::config::get_data_dir;
use color_eyre::eyre::Result;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init() -> Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(&directory)?;
    let log_file = std::fs::File::create(directory.join("app.log"))?;
    let file_layer = fmt::layer().with_writer(log_file).with_ansi(false);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(file_layer)
        .init();

    Ok(())
}
