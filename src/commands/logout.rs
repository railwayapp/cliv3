use super::*;

/// Logout of your Railway account
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args, json: bool) -> Result<()> {
    unimplemented!("logout command is not implemented yet");
    Ok(())
}
