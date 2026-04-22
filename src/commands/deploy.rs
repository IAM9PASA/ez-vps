use std::path::PathBuf;

use anyhow::Result;

use crate::{cli::DeployArgs, utils::output::print_action};

pub async fn run(_config_path: PathBuf, args: DeployArgs) -> Result<()> {
    let server = args
        .server
        .server
        .as_deref()
        .unwrap_or("<select during init>");

    print_action(&format!(
        "`deploy {}` for server '{}' is scaffolded next.",
        args.target, server
    ));
    Ok(())
}
