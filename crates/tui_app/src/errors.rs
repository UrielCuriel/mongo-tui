use color_eyre::eyre::Result;

pub fn init() -> Result<()> {
    color_eyre::install()?;
    Ok(())
}
