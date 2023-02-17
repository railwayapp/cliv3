use super::*;

/// Run a local command using variables from the active environment
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args, json: bool) -> Result<()> {
    unimplemented!("run command is not implemented yet");
    Ok(())
}
