mod auth;
mod calendar;
mod cli;
mod daemon;
mod next;
mod notify;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    console_subscriber::init();
    return match Cli::parse().command {
        Commands::Auth => auth::run().await,
        Commands::Run => daemon::run().await,
        Commands::Next => next::run().await,
    };
}
