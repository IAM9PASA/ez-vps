use std::path::PathBuf;

use anyhow::Result;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::{
    cli::UninstallArgs,
    config::{Config, ProxyType, Server},
    proxy,
    ssh::SshClient,
    utils::output::{print_action, print_kv, print_success},
};

pub async fn run(config_path: PathBuf, args: UninstallArgs) -> Result<()> {
    let mut config = Config::load(&config_path)?;
    config.validate()?;

    let server_name = args
        .server
        .server
        .clone()
        .unwrap_or_else(|| config.servers[0].name.clone());
    let server = config.find_server(&server_name)?.clone();

    if !args.server.dry_run && !args.yes && !confirm_uninstall(&server)? {
        print_action("Uninstall cancelled.");
        return Ok(());
    }

    if args.server.dry_run {
        print_action("Dry run: uninstall was not executed.");
        print_uninstall_summary(&server);
        return Ok(());
    }

    let ssh = SshClient::connect(&server).await?;
    perform_uninstall(&ssh, &server).await?;

    let server_entry = config.find_server_mut(&server_name)?;
    server_entry.apps.clear();
    server_entry.managed_docker = false;
    server_entry.managed_proxy = None;
    config.save(&config_path)?;

    print_success("Uninstall complete.");
    print_uninstall_summary(&server);
    Ok(())
}

fn confirm_uninstall(server: &Server) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Uninstall ez-vps-managed services from '{}' ({})?",
            server.name, server.host
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

async fn perform_uninstall(ssh: &SshClient, server: &Server) -> Result<()> {
    for proxy_type in proxy_types_to_remove(server) {
        if let Some(symlink) = proxy::enabled_symlink_path(proxy_type) {
            ssh.run(&format!("sudo rm -f {symlink}")).await?;
        }

        ssh.run(&format!("sudo rm -f {}", proxy::config_path(proxy_type)))
            .await?;

        let reload_command = match proxy_type {
            ProxyType::Caddy => {
                "sudo systemctl disable --now caddy || true && sudo apt remove -y caddy || true"
            }
            ProxyType::Nginx => {
                "sudo systemctl disable --now nginx || true && sudo apt remove -y nginx || true"
            }
        };

        ssh.run(reload_command).await?;
    }

    if server.managed_docker {
        ssh.run("sudo systemctl disable --now docker || true && sudo apt remove -y docker.io || true")
            .await?;
    }

    Ok(())
}

fn proxy_types_to_remove(server: &Server) -> Vec<ProxyType> {
    server.effective_proxy().into_iter().collect()
}

fn print_uninstall_summary(server: &Server) {
    print_kv("server", &server.name);
    print_kv("host", &server.host);
    print_kv("apps removed", &server.apps.len().to_string());
    print_kv("docker", "removed if present");
    print_kv("proxy", "removed if managed by ez-vps");
}

#[cfg(test)]
mod tests {
    use super::proxy_types_to_remove;
    use crate::config::{ProxyType, Server};

    #[test]
    fn removes_both_proxies_when_no_apps_are_saved() {
        let server = Server {
            name: "prod-1".into(),
            host: "203.0.113.10".into(),
            user: "root".into(),
            port: 22,
            ssh_key: "/home/root/.ssh/id_ed25519".into(),
            managed_docker: false,
            managed_proxy: None,
            apps: Vec::new(),
        };

        assert!(proxy_types_to_remove(&server).is_empty());
    }

    #[test]
    fn removes_only_tracked_proxy_when_present() {
        let server = Server {
            name: "prod-1".into(),
            host: "203.0.113.10".into(),
            user: "root".into(),
            port: 22,
            ssh_key: "/home/root/.ssh/id_ed25519".into(),
            managed_docker: false,
            managed_proxy: Some(ProxyType::Caddy),
            apps: Vec::new(),
        };

        assert_eq!(proxy_types_to_remove(&server), vec![ProxyType::Caddy]);
    }
}
