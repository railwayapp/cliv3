use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
use commands::*;

mod client;
mod config;
mod consts;
mod gql;

#[macro_use]
mod macros;

/// Interact with 🚅 Railway via CLI
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

// Generates the commands based on the modules in the commands directory
// Specify the modules you want to include in the commands_enum! macro
commands_enum!(
    add,
    completion,
    connect,
    delete,
    docs,
    down,
    environment,
    init,
    link,
    list,
    login,
    logout,
    logs,
    open,
    protect,
    run,
    shell,
    status,
    unlink,
    up,
    variables,
    whoami
);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();

    Commands::exec(cli).await?;

    Ok(())
}
