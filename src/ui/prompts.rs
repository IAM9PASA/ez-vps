use anyhow::{Result, bail};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

use crate::config::{Config, Server};

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
    let proxy_config = collect_proxy_config(proxy, is_interactive)?;
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

    let host: String = Input::with_theme(&theme)
        .with_prompt("Host")
        .interact_text()?;
    let user: String = Input::with_theme(&theme)
        .with_prompt("User")
        .default("root".into())
        .interact_text()?;
    let port: u16 = Input::with_theme(&theme)
        .with_prompt("Port")
        .default(22)
        .interact_text()?;
    let ssh_key: String = Input::with_theme(&theme)
        .with_prompt("SSH key path")
        .interact_text()?;

    Ok(Server {
        name: "ad-hoc".into(),
        host,
        user,
        port,
        ssh_key,
        apps: Vec::new(),
    })
}

fn select_proxy(is_interactive: bool) -> Result<ProxyKind> {
    if !is_interactive {
        return Ok(ProxyKind::Caddy);
    }

    let theme = ColorfulTheme::default();
    let options = [
        "Caddy (recommended)",
        "Nginx",
        "None",
    ];

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

fn collect_proxy_config(proxy: ProxyKind, is_interactive: bool) -> Result<Option<ProxyConfig>> {
    if proxy == ProxyKind::None {
        return Ok(None);
    }

    if !is_interactive {
        return Ok(Some(ProxyConfig {
            domain: "example.com".into(),
            upstream_port: 3000,
        }));
    }

    let theme = ColorfulTheme::default();
    let domain: String = Input::with_theme(&theme)
        .with_prompt("Domain for the proxy")
        .with_initial_text("example.com")
        .interact_text()?;
    let upstream_port: u16 = Input::with_theme(&theme)
        .with_prompt("App port to proxy to")
        .default(3000)
        .interact_text()?;

    Ok(Some(ProxyConfig {
        domain,
        upstream_port,
    }))
}
