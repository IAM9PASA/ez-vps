use console::style;

use crate::ui::prompts::{InitOptions, ProxyKind};

pub fn print_init_summary(options: &InitOptions) {
    println!();
    println!("{}", style("Ready to apply").bold().underlined());
    println!("  Server: {}", options.server.name);
    println!("  Target: {}", options.server.destination());
    println!("  User: {}", options.server.user);
    println!("  Port: {}", options.server.port);
    println!("  SSH key: {}", options.server.ssh_key);
    println!("  Proxy: {}", options.proxy.label());
    println!(
        "  Docker: {}",
        if options.install_docker { "Yes" } else { "No" }
    );

    if let Some(proxy_config) = &options.proxy_config {
        println!("  Domain: {}", proxy_config.domain);
        println!("  Upstream: 127.0.0.1:{}", proxy_config.upstream_port);
    }

    println!();

    if options.proxy != ProxyKind::None {
        println!("{}", style("Generated proxy config").bold());
        println!("{}", proxy_config_preview(options));
        println!();
    }
}

fn proxy_config_preview(options: &InitOptions) -> String {
    let Some(proxy_config) = &options.proxy_config else {
        return String::new();
    };

    match options.proxy {
        ProxyKind::Caddy => format!(
            "{} {{\n    reverse_proxy 127.0.0.1:{}\n}}\n",
            proxy_config.domain, proxy_config.upstream_port
        ),
        ProxyKind::Nginx => format!(
            "server {{\n    listen 80;\n    server_name {};\n\n    location / {{\n        proxy_pass http://127.0.0.1:{};\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto $scheme;\n    }}\n}}\n",
            proxy_config.domain, proxy_config.upstream_port
        ),
        ProxyKind::None => String::new(),
    }
}
