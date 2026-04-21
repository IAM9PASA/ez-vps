use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    cli::VersionArgs,
    utils::output::{print_action, print_kv, print_success},
};

const REPO_SLUG: &str = "IAM9PASA/ez-vps";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

pub async fn run(args: VersionArgs) -> Result<()> {
    print_kv("installed", CURRENT_VERSION);

    if !args.check_latest {
        return Ok(());
    }

    print_action("Checking latest GitHub release");
    let release = fetch_latest_release().await?;
    let latest = release.tag_name.trim_start_matches('v');

    print_kv("latest", latest);
    print_kv("release", &release.html_url);

    if latest == CURRENT_VERSION {
        print_success("You are on the latest release.");
    } else {
        print_action("A newer release is available.");
    }

    Ok(())
}

async fn fetch_latest_release() -> Result<Release> {
    let url = format!("https://api.github.com/repos/{REPO_SLUG}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent(format!("ez-vps/{CURRENT_VERSION}"))
        .build()
        .context("failed to build HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .context("failed to query GitHub releases API")?
        .error_for_status()
        .context("GitHub releases API returned an error status")?;

    response
        .json::<Release>()
        .await
        .context("failed to parse GitHub release response")
}
