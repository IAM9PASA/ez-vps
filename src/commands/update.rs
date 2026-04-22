use std::path::PathBuf;

use anyhow::Result;

use crate::{
    cli::ServerArgs,
    config::{Config, ProxyType, Server},
    proxy,
    ssh::SshClient,
    utils::output::{print_action, print_kv, print_success},
};

pub async fn run(config_path: PathBuf, args: ServerArgs) -> Result<()> {
    let config = Config::load(&config_path)?;
    config.validate()?;

    let server_name = args
        .server
        .clone()
        .unwrap_or_else(|| config.servers[0].name.clone());
    let server = config.find_server(&server_name)?;

    print_action(&format!("Updating server '{}'", server.name));

    if args.dry_run {
        print_action("Dry run: package updates and proxy re-apply were skipped.");
        print_update_summary(server);
        return Ok(());
    }

    let ssh = SshClient::connect(server).await?;
    apply_update(&ssh, server).await?;

    print_success("Server update complete.");
    print_update_summary(server);
    Ok(())
}

async fn apply_update(ssh: &SshClient, server: &Server) -> Result<()> {
    ssh.run("sudo apt update").await?;
    ssh.run("sudo apt upgrade -y").await?;
    ssh.run("sudo apt install -y curl ufw").await?;

    if server.managed_docker {
        ssh.run("sudo apt install -y docker.io").await?;
        ssh.run("sudo systemctl enable --now docker").await?;
    }

    if let Some(proxy_type) = server_proxy(server) {
        ssh.run(proxy::install_command(proxy_type)).await?;

        let content = proxy::render_proxy_config(server, proxy_type);
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
    }

    Ok(())
}

fn server_proxy(server: &Server) -> Option<ProxyType> {
    server.effective_proxy()
}

fn print_update_summary(server: &Server) {
    print_kv("server", &server.name);
    print_kv("host", &server.host);
    print_kv("apps", &server.apps.len().to_string());
    print_kv(
        "proxy",
        server_proxy(server).map(|proxy| proxy.label()).unwrap_or("none"),
    );
}

#[cfg(test)]
mod tests {
    use super::server_proxy;
    use crate::config::{App, ProxyType, Server};

    #[test]
    fn detects_server_proxy_from_first_app() {
        let server = Server {
            name: "prod-1".into(),
            host: "203.0.113.10".into(),
            user: "root".into(),
            port: 22,
            ssh_key: "/home/root/.ssh/id_ed25519".into(),
            managed_docker: false,
            managed_proxy: None,
            apps: vec![App {
                domain: "api.example.com".into(),
                upstream_port: 8000,
                proxy: ProxyType::Caddy,
            }],
        };

        assert_eq!(server_proxy(&server), Some(ProxyType::Caddy));
    }

    #[test]
    fn prefers_saved_managed_proxy_without_apps() {
        let server = Server {
            name: "prod-1".into(),
            host: "203.0.113.10".into(),
            user: "root".into(),
            port: 22,
            ssh_key: "/home/root/.ssh/id_ed25519".into(),
            managed_docker: true,
            managed_proxy: Some(ProxyType::Nginx),
            apps: Vec::new(),
        };

        assert_eq!(server_proxy(&server), Some(ProxyType::Nginx));
    }
}
