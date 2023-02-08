use super::*;

/// Get the version of the Railway CLI
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    unimplemented!("version command is not implemented yet");
    Ok(())
}
