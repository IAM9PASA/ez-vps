use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub servers: Vec<Server>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub user: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub ssh_key: String,
    #[serde(default)]
    pub managed_docker: bool,
    #[serde(default)]
    pub managed_proxy: Option<ProxyType>,
    #[serde(default)]
    pub apps: Vec<App>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub domain: String,
    pub upstream_port: u16,
    pub proxy: ProxyType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Caddy,
    Nginx,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file at {}", path.display()))?;

        toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML from {}", path.display()))
    }

    pub fn find_server(&self, name: &str) -> Result<&Server> {
        self.servers
            .iter()
            .find(|server| server.name == name)
            .ok_or_else(|| anyhow::anyhow!("server '{name}' was not found in config"))
    }

    pub fn validate(&self) -> Result<()> {
        if self.servers.is_empty() {
            bail!("config does not contain any [[servers]] entries");
        }

        Ok(())
    }

    pub fn find_server_mut(&mut self, name: &str) -> Result<&mut Server> {
        self.servers
            .iter_mut()
            .find(|server| server.name == name)
            .ok_or_else(|| anyhow::anyhow!("server '{name}' was not found in config"))
    }

    pub fn upsert_server(&mut self, server: Server) {
        if let Some(existing) = self
            .servers
            .iter_mut()
            .find(|item| item.name == server.name)
        {
            *existing = server;
        } else {
            self.servers.push(server);
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let raw = toml::to_string_pretty(self).context("failed to serialize config to TOML")?;

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create config directory {}", parent.display())
                })?;
            }
        }

        fs::write(path, raw)
            .with_context(|| format!("failed to write config file to {}", path.display()))
    }
}

impl Server {
    pub fn destination(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    pub fn upsert_app(&mut self, app: App) {
        if let Some(existing) = self.apps.iter_mut().find(|item| item.domain == app.domain) {
            *existing = app;
        } else {
            self.apps.push(app);
        }
    }

    pub fn effective_proxy(&self) -> Option<ProxyType> {
        self.managed_proxy
            .or_else(|| self.apps.first().map(|app| app.proxy))
    }
}

impl ProxyType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Caddy => "Caddy",
            Self::Nginx => "Nginx",
        }
    }
}

fn default_ssh_port() -> u16 {
    22
}
