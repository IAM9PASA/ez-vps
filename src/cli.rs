use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "ez-vps",
    version,
    about = "Simple VPS setup and checks over SSH"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "servers.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(ServerArgs),
    Update(ServerArgs),
    Uninstall(UninstallArgs),
    Check(ServerArgs),
    Version(VersionArgs),
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
    Deploy(DeployArgs),
    Status(ServerArgs),
}

#[derive(Debug, Subcommand)]
pub enum AppCommands {
    Add(AppAddArgs),
    List(ServerArgs),
    Remove(AppRemoveArgs),
}

#[derive(Debug, clap::Args)]
pub struct ServerArgs {
    #[arg(long)]
    pub server: Option<String>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    #[command(flatten)]
    pub server: ServerArgs,

    #[arg(long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Debug, clap::Args)]
pub struct DeployArgs {
    #[arg(value_name = "TARGET")]
    pub target: String,

    #[command(flatten)]
    pub server: ServerArgs,
}

#[derive(Debug, clap::Args)]
pub struct AppAddArgs {
    #[command(flatten)]
    pub server: ServerArgs,

    #[arg(long)]
    pub domain: Option<String>,

    #[arg(long)]
    pub upstream_port: Option<u16>,

    #[arg(long, value_enum)]
    pub proxy: Option<ProxyValue>,
}

#[derive(Debug, clap::Args)]
pub struct AppRemoveArgs {
    #[command(flatten)]
    pub server: ServerArgs,

    #[arg(long)]
    pub domain: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct VersionArgs {
    #[arg(long, default_value_t = false)]
    pub check_latest: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ProxyValue {
    Caddy,
    Nginx,
}
