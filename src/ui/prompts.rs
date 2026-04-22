use std::{env, path::PathBuf};

use anyhow::{Result, bail};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

use crate::config::{App, Config, Server};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyKind {
    Caddy,
    Nginx,
    None,
}

impl ProxyKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Caddy => "Caddy",
            Self::Nginx => "Nginx",
            Self::None => "None",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub domain: String,
    pub upstream_port: u16,
}

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub server: Server,
    pub proxy: ProxyKind,
    pub proxy_config: Option<ProxyConfig>,
    pub install_docker: bool,
}

pub fn collect_init_options(
    config: &Config,
    requested_server: Option<&str>,
    is_interactive: bool,
) -> Result<InitOptions> {
    let server = select_server(config, requested_server, is_interactive)?;
    let proxy = select_proxy(is_interactive)?;
    let proxy_config = collect_proxy_config(proxy, &server, is_interactive)?;
    let install_docker = select_docker(is_interactive)?;

    Ok(InitOptions {
        server,
        proxy,
        proxy_config,
        install_docker,
    })
}

fn select_server(
    config: &Config,
    requested_server: Option<&str>,
    is_interactive: bool,
) -> Result<Server> {
    if let Some(name) = requested_server {
        return Ok(config.find_server(name)?.clone());
    }

    if !is_interactive {
        bail!("`init` requires --server when prompts are disabled");
    }

    if config.servers.is_empty() {
        return prompt_ad_hoc_server();
    }

    if config.servers.len() == 1 {
        return Ok(config.servers[0].clone());
    }

    let theme = ColorfulTheme::default();
    let items: Vec<String> = config
        .servers
        .iter()
        .map(|server| format!("{} ({})", server.name, server.host))
        .collect();

    let index = Select::with_theme(&theme)
        .with_prompt("Select server")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(config.servers[index].clone())
}

fn prompt_ad_hoc_server() -> Result<Server> {
    let theme = ColorfulTheme::default();
    let default_user = detect_current_user();
    let default_ssh_key = infer_default_ssh_key_path();

    let name: String = Input::with_theme(&theme)
        .with_prompt("Server name")
        .default("prod-1".into())
        .interact_text()?;
    let host: String = Input::with_theme(&theme)
        .with_prompt("Host (public VPS IP or hostname)")
        .interact_text()?;
    let user: String = Input::with_theme(&theme)
        .with_prompt("User")
        .default(default_user)
        .interact_text()?;
    let port: u16 = Input::with_theme(&theme)
        .with_prompt("Port")
        .default(22)
        .interact_text()?;
    let ssh_key: String = Input::with_theme(&theme)
        .with_prompt("SSH key path (private key on the machine running ez-vps)")
        .default(default_ssh_key)
        .interact_text()?;

    Ok(Server {
        name,
        host,
        user,
        port,
        ssh_key,
        managed_docker: false,
        managed_proxy: None,
        apps: Vec::new(),
    })
}

fn select_proxy(is_interactive: bool) -> Result<ProxyKind> {
    if !is_interactive {
        return Ok(ProxyKind::Caddy);
    }

    let theme = ColorfulTheme::default();
    let options = ["Caddy (recommended)", "Nginx", "None"];

    let index = Select::with_theme(&theme)
        .with_prompt("Choose reverse proxy")
        .items(&options)
        .default(0)
        .interact()?;

    let proxy = match index {
        0 => ProxyKind::Caddy,
        1 => ProxyKind::Nginx,
        _ => ProxyKind::None,
    };

    Ok(proxy)
}

fn select_docker(is_interactive: bool) -> Result<bool> {
    if !is_interactive {
        return Ok(true);
    }

    let theme = ColorfulTheme::default();
    Confirm::with_theme(&theme)
        .with_prompt("Install Docker?")
        .default(true)
        .interact()
        .map_err(Into::into)
}

fn collect_proxy_config(
    proxy: ProxyKind,
    server: &Server,
    is_interactive: bool,
) -> Result<Option<ProxyConfig>> {
    let default_domain = default_domain(server);
    let default_upstream_port = default_upstream_port(server);

    if proxy == ProxyKind::None {
        return Ok(None);
    }

    if !is_interactive {
        return Ok(Some(ProxyConfig {
            domain: default_domain,
            upstream_port: default_upstream_port,
        }));
    }

    let theme = ColorfulTheme::default();
    let domain: String = Input::with_theme(&theme)
        .with_prompt("Domain for the proxy (full hostname like gg.example.com)")
        .with_initial_text(default_domain)
        .interact_text()?;
    let upstream_port: u16 = Input::with_theme(&theme)
        .with_prompt("App port to proxy to (the port your app already listens on)")
        .default(default_upstream_port)
        .interact_text()?;

    Ok(Some(ProxyConfig {
        domain,
        upstream_port,
    }))
}

fn detect_current_user() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "root".into())
}

fn infer_default_ssh_key_path() -> String {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from));

    let Some(home) = home else {
        return "~/.ssh/id_ed25519".into();
    };

    let candidates = [
        home.join(".ssh").join("id_ed25519"),
        home.join(".ssh").join("id_rsa"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    home.join(".ssh")
        .join("id_ed25519")
        .to_string_lossy()
        .into_owned()
}

fn default_domain(server: &Server) -> String {
    first_existing_app(server)
        .map(|app| app.domain.clone())
        .unwrap_or_else(|| "example.com".into())
}

fn default_upstream_port(server: &Server) -> u16 {
    first_existing_app(server)
        .map(|app| app.upstream_port)
        .unwrap_or(8000)
}

fn first_existing_app(server: &Server) -> Option<&App> {
    server.apps.first()
}

#[cfg(test)]
mod tests {
    use super::{default_upstream_port, infer_default_ssh_key_path};
    use crate::config::{App, ProxyType, Server};

    #[test]
    fn defaults_to_web_app_port_when_server_has_no_apps() {
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

        assert_eq!(default_upstream_port(&server), 8000);
    }

    #[test]
    fn reuses_existing_app_port_when_present() {
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
                upstream_port: 9000,
                proxy: ProxyType::Caddy,
            }],
        };

        assert_eq!(default_upstream_port(&server), 9000);
    }

    #[test]
    fn inferred_ssh_key_path_is_not_empty() {
        assert!(!infer_default_ssh_key_path().trim().is_empty());
    }
}
