use super::*;

/// View the most-recent deploy's logs
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    unimplemented!("logs command is not implemented yet");
}
