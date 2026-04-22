use crate::config::{App, ProxyType, Server};

pub fn render_proxy_config(server: &Server, proxy: ProxyType) -> String {
    match proxy {
        ProxyType::Caddy => render_caddy_config(server),
        ProxyType::Nginx => render_nginx_config(server),
    }
}

pub fn config_path(proxy: ProxyType) -> &'static str {
    match proxy {
        ProxyType::Caddy => "/etc/caddy/Caddyfile",
        ProxyType::Nginx => "/etc/nginx/sites-available/ez-vps.conf",
    }
}

pub fn enabled_symlink_path(proxy: ProxyType) -> Option<&'static str> {
    match proxy {
        ProxyType::Caddy => None,
        ProxyType::Nginx => Some("/etc/nginx/sites-enabled/ez-vps.conf"),
    }
}

pub fn reload_command(proxy: ProxyType) -> &'static str {
    match proxy {
        ProxyType::Caddy => "sudo systemctl reload caddy",
        ProxyType::Nginx => "sudo nginx -t && sudo systemctl reload nginx",
    }
}

pub fn install_command(proxy: ProxyType) -> &'static str {
    match proxy {
        ProxyType::Caddy => "sudo apt install -y caddy",
        ProxyType::Nginx => "sudo apt install -y nginx",
    }
}

pub fn render_app_preview(app: &App) -> String {
    match app.proxy {
        ProxyType::Caddy => format!(
            "{} {{\n    reverse_proxy 127.0.0.1:{}\n}}\n",
            app.domain, app.upstream_port
        ),
        ProxyType::Nginx => format!(
            "server {{\n    listen 80;\n    server_name {};\n\n    location / {{\n        proxy_pass http://127.0.0.1:{};\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n    }}\n}}\n",
            app.domain, app.upstream_port
        ),
    }
}

fn render_caddy_config(server: &Server) -> String {
    if server.apps.is_empty() {
        return ":80 {\n    respond \"ez-vps is ready\" 200\n}\n".to_string();
    }

    server
        .apps
        .iter()
        .filter(|app| app.proxy == ProxyType::Caddy)
        .map(render_app_preview)
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_nginx_config(server: &Server) -> String {
    let apps: Vec<&App> = server
        .apps
        .iter()
        .filter(|app| app.proxy == ProxyType::Nginx)
        .collect();

    if apps.is_empty() {
        return "server {\n    listen 80 default_server;\n    server_name _;\n\n    return 200 'ez-vps is ready';\n}\n".to_string();
    }

    apps.iter()
        .map(|app| render_app_preview(app))
        .collect::<Vec<_>>()
        .join("\n")
}
