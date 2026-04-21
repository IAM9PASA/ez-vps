use std::path::PathBuf;

use anyhow::{Result, bail};
use dialoguer::{Input, Select, theme::ColorfulTheme};

use crate::{
    cli::{AppAddArgs, AppCommands, AppRemoveArgs, ProxyValue, ServerArgs},
    config::{App, Config, ProxyType, Server},
    proxy,
    ssh::SshClient,
    utils::output::{print_action, print_kv, print_success},
};

pub async fn run(config_path: PathBuf, command: AppCommands) -> Result<()> {
    match command {
        AppCommands::Add(args) => add(config_path, args).await,
        AppCommands::List(args) => list(config_path, args).await,
        AppCommands::Remove(args) => remove(config_path, args).await,
    }
}

async fn add(config_path: PathBuf, args: AppAddArgs) -> Result<()> {
    let mut config = Config::load(&config_path)?;
    config.validate()?;

    let server_name = resolve_server_name(&config, &args.server)?;
    let dry_run = args.server.dry_run;
    let app;
    let server_snapshot;
    {
        let server = config.find_server_mut(&server_name)?;
        app = collect_app_args(server, &args)?;

        if server.apps.iter().any(|existing| existing.domain == app.domain) {
            bail!(
                "an app for domain '{}' already exists on server '{}'",
                app.domain,
                server.name
            );
        }

        if let Some(existing_proxy) = default_proxy_for_server(server) {
            if existing_proxy != app.proxy {
                bail!(
                    "server '{}' already uses {}. mixing proxy types on one server is not supported yet",
                    server.name,
                    existing_proxy.label()
                );
            }
        }

        server.apps.push(app.clone());
        server_snapshot = server.clone();
    }

    if dry_run {
        print_action("Dry run: app mapping was not saved.");
    } else {
        apply_proxy(&server_snapshot).await?;
        config.save(&config_path)?;
        print_success("App mapping saved and proxy config applied.");
    }

    print_kv("server", &server_snapshot.name);
    print_kv("domain", &app.domain);
    print_kv("proxy", app.proxy.label());
    print_kv("upstream", &format!("127.0.0.1:{}", app.upstream_port));

    Ok(())
}

async fn list(config_path: PathBuf, args: ServerArgs) -> Result<()> {
    let config = Config::load(&config_path)?;
    config.validate()?;

    let server_name = resolve_server_name(&config, &args)?;
    let server = config.find_server(&server_name)?;

    print_action(&format!("Apps for server '{}'", server.name));

    if server.apps.is_empty() {
        print_action("No app mappings saved yet.");
        return Ok(());
    }

    for app in &server.apps {
        println!(
            "  - {} -> 127.0.0.1:{} ({})",
            app.domain,
            app.upstream_port,
            app.proxy.label()
        );
    }

    Ok(())
}

async fn remove(config_path: PathBuf, args: AppRemoveArgs) -> Result<()> {
    let mut config = Config::load(&config_path)?;
    config.validate()?;

    let server_name = resolve_server_name(&config, &args.server)?;
    let dry_run = args.server.dry_run;
    let removed_domain;
    let server_snapshot;
    {
        let server = config.find_server_mut(&server_name)?;
        removed_domain = resolve_domain_for_removal(server, args.domain.as_deref())?.to_string();

        let previous_len = server.apps.len();
        server.apps.retain(|app| app.domain != removed_domain);

        if server.apps.len() == previous_len {
            bail!(
                "domain '{}' was not found on server '{}'",
                removed_domain,
                server.name
            );
        }

        server_snapshot = server.clone();
    }

    if dry_run {
        print_action("Dry run: app mapping was not removed.");
    } else {
        apply_proxy(&server_snapshot).await?;
        config.save(&config_path)?;
        print_success("App mapping removed and proxy config applied.");
    }

    print_kv("server", &server_snapshot.name);
    print_kv("domain", &removed_domain);

    Ok(())
}

fn resolve_server_name(config: &Config, args: &ServerArgs) -> Result<String> {
    if let Some(server) = &args.server {
        return Ok(server.clone());
    }

    if config.servers.len() == 1 {
        return Ok(config.servers[0].name.clone());
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

    Ok(config.servers[index].name.clone())
}

fn collect_app_args(server: &Server, args: &AppAddArgs) -> Result<App> {
    let theme = ColorfulTheme::default();

    let domain = match &args.domain {
        Some(domain) => domain.clone(),
        None => Input::with_theme(&theme)
            .with_prompt("Domain")
            .interact_text()?,
    };

    let upstream_port = match args.upstream_port {
        Some(domain) => domain,
        None => Input::with_theme(&theme)
            .with_prompt("App port")
            .default(3000)
            .interact_text()?,
    };

    let proxy = match args.proxy {
        Some(proxy) => proxy.into(),
        None => default_proxy_for_server(server).unwrap_or_else(|| select_proxy(&theme)),
    };

    Ok(App {
        domain,
        upstream_port,
        proxy,
    })
}

fn default_proxy_for_server(server: &Server) -> Option<ProxyType> {
    server.apps.first().map(|app| app.proxy)
}

fn select_proxy(theme: &ColorfulTheme) -> ProxyType {
    let options = ["Caddy", "Nginx"];
    let index = Select::with_theme(theme)
        .with_prompt("Proxy")
        .items(&options)
        .default(0)
        .interact()
        .unwrap_or(0);

    match index {
        0 => ProxyType::Caddy,
        _ => ProxyType::Nginx,
    }
}

fn resolve_domain_for_removal<'a>(server: &'a Server, provided: Option<&'a str>) -> Result<&'a str> {
    if let Some(domain) = provided {
        return Ok(domain);
    }

    if server.apps.is_empty() {
        bail!("server '{}' has no app mappings to remove", server.name);
    }

    if server.apps.len() == 1 {
        return Ok(&server.apps[0].domain);
    }

    let theme = ColorfulTheme::default();
    let items: Vec<String> = server
        .apps
        .iter()
        .map(|app| format!("{} -> 127.0.0.1:{}", app.domain, app.upstream_port))
        .collect();

    let index = Select::with_theme(&theme)
        .with_prompt("Select app to remove")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(&server.apps[index].domain)
}

async fn apply_proxy(server: &Server) -> Result<()> {
    let proxy = dominant_proxy(server);
    let ssh = SshClient::connect(server).await?;

    ssh.run("sudo apt update").await?;
    ssh.run(proxy::install_command(proxy)).await?;

    let content = proxy::render_proxy_config(server, proxy);
    ssh.write_file(proxy::config_path(proxy), &content).await?;

    if let Some(symlink) = proxy::enabled_symlink_path(proxy) {
        ssh.run(&format!(
            "sudo ln -sf {} {}",
            proxy::config_path(proxy),
            symlink
        ))
        .await?;
    }

    ssh.run(proxy::reload_command(proxy)).await?;
    Ok(())
}

fn dominant_proxy(server: &Server) -> ProxyType {
    server
        .apps
        .first()
        .map(|app| app.proxy)
        .unwrap_or(ProxyType::Caddy)
}

impl From<ProxyValue> for ProxyType {
    fn from(value: ProxyValue) -> Self {
        match value {
            ProxyValue::Caddy => ProxyType::Caddy,
            ProxyValue::Nginx => ProxyType::Nginx,
        }
    }
}
