use anyhow::bail;

use super::*;

/// Open Railway Documentation in default browser
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    let config = Configs::new()?;
    let confirm = inquire::Confirm::new("Open the browser")
        .with_default(true)
        .with_render_config(config.get_render_config())
        .prompt()?;

    if !confirm {
        bail!("Aborted by user");
    }

    ::open::that("https://docs.railway.app/")?;
    Ok(())
}
