use std::path::PathBuf;

use anyhow::{Result, bail};
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::{
    cli::ServerArgs,
    config::{App, Config, ProxyType},
    proxy,
    ssh::SshClient,
    ui::{
        progress::print_planned_commands,
        prompts::collect_init_options,
        summary::print_init_summary,
    },
    utils::output::{print_action, print_kv, print_success},
};

pub async fn run(config_path: PathBuf, args: ServerArgs) -> Result<()> {
    print_action(&format!("Loading config from {}", config_path.display()));
    let config = Config::load(&config_path)?;
    let interactive = args.server.is_none();

    if !config.servers.is_empty() {
        config.validate()?;
    }

    let options = collect_init_options(&config, args.server.as_deref(), interactive)?;

    print_init_summary(&options);
    print_planned_commands(&options);

    if args.dry_run {
        print_success("Dry run complete.");
        return Ok(());
    }

    if interactive {
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed?")
            .default(true)
            .interact()?;

        if !proceed {
            print_action("Init cancelled.");
            return Ok(());
        }
    }

    print_action(&format!("Connecting to {}", options.server.destination()));
    let ssh = SshClient::connect(&options.server).await?;

    print_action("Detecting remote distro");
    let distro = ssh.detect_distro().await?;
    if !distro.starts_with("ubuntu ") {
        bail!("unsupported distro '{distro}'. Ubuntu 22.04+ is the current supported target");
    }

    let proxy_type = match options.proxy {
        crate::ui::prompts::ProxyKind::Caddy => Some(ProxyType::Caddy),
        crate::ui::prompts::ProxyKind::Nginx => Some(ProxyType::Nginx),
        crate::ui::prompts::ProxyKind::None => None,
    };

    execute_base_setup(&ssh, &options, proxy_type).await?;

    print_success("Server initialized successfully.");
    print_kv("server", &options.server.name);
    print_kv("distro", &distro);

    Ok(())
}

async fn execute_base_setup(
    ssh: &SshClient,
    options: &crate::ui::prompts::InitOptions,
    proxy_type: Option<ProxyType>,
) -> Result<()> {
    print_action("Updating packages");
    ssh.run("sudo apt update").await?;

    print_action("Installing base packages");
    ssh.run("sudo apt install -y curl ufw").await?;

    if options.install_docker {
        print_action("Installing Docker");
        ssh.run("sudo apt install -y docker.io").await?;
        ssh.run("sudo systemctl enable --now docker").await?;
    }

    if let Some(proxy_type) = proxy_type {
        print_action(&format!("Installing {}", proxy_type.label()));
        ssh.run(proxy::install_command(proxy_type)).await?;

        let mut server = options.server.clone();
        if let Some(proxy_config) = &options.proxy_config {
            server.apps.push(App {
                domain: proxy_config.domain.clone(),
                upstream_port: proxy_config.upstream_port,
                proxy: proxy_type,
            });
        }

        let content = proxy::render_proxy_config(&server, proxy_type);
        ssh.write_file(proxy::config_path(proxy_type), &content).await?;

        if let Some(symlink) = proxy::enabled_symlink_path(proxy_type) {
            ssh.run(&format!(
                "sudo ln -sf {} {}",
                proxy::config_path(proxy_type),
                symlink
            ))
            .await?;
        }

        ssh.run(proxy::reload_command(proxy_type)).await?;
        ssh.run("sudo ufw allow 80").await?;
        ssh.run("sudo ufw allow 443").await?;
    }

    print_action("Configuring firewall");
    ssh.run("sudo ufw allow 22").await?;
    ssh.run("sudo ufw --force enable").await?;

    Ok(())
}
