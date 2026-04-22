use std::path::PathBuf;

use anyhow::Result;

use crate::{
    cli::ServerArgs,
    config::{Config, ProxyType},
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
    let ssh = SshClient::connect(server).await?;

    print_action(&format!("Running checks against {}", server.destination()));

    let distro = ssh.detect_distro().await?;
    let docker = check_command(&ssh, "docker --version").await;
    let ufw = check_command(&ssh, "sudo ufw status").await;

    print_kv("distro", &distro);
    print_kv("docker", if docker { "ok" } else { "missing" });
    print_kv("firewall", if ufw { "ok" } else { "missing" });

    let proxies = proxy_types_for_server(server);
    for proxy_type in proxies {
        let installed = match proxy_type {
            ProxyType::Caddy => check_command(&ssh, "caddy version").await,
            ProxyType::Nginx => check_command(&ssh, "nginx -v").await,
        };

        print_kv(
            proxy_type.label(),
            if installed { "installed" } else { "missing" },
        );
    }

    if server.apps.is_empty() {
        print_action("No saved apps to probe.");
    } else {
        for app in &server.apps {
            let command = format!(
                "curl -I -H 'Host: {}' http://127.0.0.1 2>/dev/null | head -n 1",
                app.domain
            );
            let reachable = ssh.run(&command).await.unwrap_or_default();
            print_kv(
                &format!("app {}", app.domain),
                if is_successful_probe_response(&reachable) {
                    "reachable"
                } else {
                    "not reachable"
                },
            );
        }
    }

    print_success("Check complete.");
    Ok(())
}

async fn check_command(ssh: &SshClient, command: &str) -> bool {
    ssh.run(command).await.is_ok()
}

fn is_successful_probe_response(response: &str) -> bool {
    response
        .split_whitespace()
        .find_map(|token| token.parse::<u16>().ok())
        .map(|status| matches!(status, 200..=399))
        .unwrap_or(false)
}

fn proxy_types_for_server(server: &crate::config::Server) -> Vec<ProxyType> {
    let mut proxies = Vec::new();

    for app in &server.apps {
        if !proxies.contains(&app.proxy) {
            proxies.push(app.proxy);
        }
    }

    if proxies.is_empty() {
        proxies.push(ProxyType::Caddy);
    }

    proxies
}

#[cfg(test)]
mod tests {
    use super::is_successful_probe_response;

    #[test]
    fn accepts_redirect_responses() {
        assert!(is_successful_probe_response("HTTP/1.1 308 Permanent Redirect"));
    }

    #[test]
    fn rejects_server_errors() {
        assert!(!is_successful_probe_response("HTTP/1.1 502 Bad Gateway"));
    }
}
