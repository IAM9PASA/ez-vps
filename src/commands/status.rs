use std::path::PathBuf;

use anyhow::Result;

use crate::{cli::ServerArgs, utils::output::print_action};

pub async fn run(_config_path: PathBuf, args: ServerArgs) -> Result<()> {
    let server = args.server.as_deref().unwrap_or("<select during init>");

    print_action(&format!(
        "`status` for server '{}' is scaffolded next.",
        server
    ));
    Ok(())
}
