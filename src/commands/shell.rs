use super::*;

/// Open a subshell with Railway variables available
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args, json: bool) -> Result<()> {
    unimplemented!("shell command is not implemented yet");
    Ok(())
}
