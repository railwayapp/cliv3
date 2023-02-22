use anyhow::Result;
use clap::Parser;

pub mod client;
pub(crate) mod config;
pub(crate) mod consts;
pub(crate) mod entities;
pub mod gql;
// mod subscription;
pub(crate) mod table;
mod commands;

#[macro_use]
mod macros;

mod cli;
use cli::*;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();

    Commands::exec(cli).await?;

    Ok(())
}
