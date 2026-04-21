use crate::{
    config::{App, ProxyType},
    proxy,
    ui::prompts::{InitOptions, ProxyKind},
};

pub fn planned_commands(options: &InitOptions) -> Vec<String> {
    let mut commands = vec![
        "sudo apt update".to_string(),
        "sudo apt install -y curl ufw".to_string(),
    ];

    if options.install_docker {
        commands.push("sudo apt install -y docker.io".to_string());
        commands.push("sudo systemctl enable --now docker".to_string());
    }

    match options.proxy {
        ProxyKind::Caddy => {
            append_proxy_plan(&mut commands, options, ProxyType::Caddy);
        }
        ProxyKind::Nginx => {
            append_proxy_plan(&mut commands, options, ProxyType::Nginx);
        }
        ProxyKind::None => {}
    }

    commands.push("sudo ufw allow 22".to_string());

    if options.proxy != ProxyKind::None {
        commands.push("sudo ufw allow 80".to_string());
        commands.push("sudo ufw allow 443".to_string());
    }

    commands.push("sudo ufw --force enable".to_string());

    commands
}

fn append_proxy_plan(commands: &mut Vec<String>, options: &InitOptions, proxy_type: ProxyType) {
    commands.push(proxy::install_command(proxy_type).to_string());

    let app = options.proxy_config.as_ref().map(|cfg| App {
        domain: cfg.domain.clone(),
        upstream_port: cfg.upstream_port,
        proxy: proxy_type,
    });

    if let Some(app) = app {
        commands.push(format!(
            "write {} for {} -> 127.0.0.1:{}",
            proxy::config_path(proxy_type),
            app.domain,
            app.upstream_port
        ));
    }

    if let Some(path) = proxy::enabled_symlink_path(proxy_type) {
        commands.push(format!(
            "sudo ln -sf {} {}",
            proxy::config_path(proxy_type),
            path
        ));
    }

    commands.push(proxy::reload_command(proxy_type).to_string());
}

pub fn print_planned_commands(options: &InitOptions) {
    println!("Planned commands:");

    for command in planned_commands(options) {
        println!("  - {command}");
    }
}
