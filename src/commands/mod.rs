mod app;
mod check;
mod deploy;
mod init;
mod status;

use anyhow::Result;

use crate::cli::{Cli, Commands};

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init(args) => init::run(cli.config, args).await,
        Commands::Check(args) => check::run(cli.config, args).await,
        Commands::App { command } => app::run(cli.config, command).await,
        Commands::Deploy(args) => deploy::run(cli.config, args).await,
        Commands::Status(args) => status::run(cli.config, args).await,
    }
}
