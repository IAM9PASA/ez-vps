mod cli;
mod commands;
mod config;
mod error;
mod proxy;
mod ssh;
mod ui;
mod utils;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::error::ResultExt;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = cli::Cli::parse();
    commands::run(cli).await.print_and_exit()
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
