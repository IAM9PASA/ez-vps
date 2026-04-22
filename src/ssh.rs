use std::{
    env,
    net::ToSocketAddrs,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{Context, Result, bail};
use tokio::process::Command;

use crate::config::Server;

pub struct SshClient {
    server: Server,
}

impl SshClient {
    pub async fn connect(server: &Server) -> Result<Self> {
        validate_local_ssh_key(server)?;

        let client = Self {
            server: server.clone(),
        };

        if let Err(error) = client.run("echo connected").await {
            let diagnosis = diagnose_connection_error(server, &error).await;
            return Err(error).context(diagnosis).context(format!(
                "failed to connect to {} using local ssh client",
                server.destination()
            ));
        }

        Ok(client)
    }

    pub async fn run(&self, command: &str) -> Result<String> {
        let remote_command = format!("sh -lc {}", shell_quote(command));
        let output = Command::new("ssh")
            .arg("-i")
            .arg(resolve_ssh_key_path(&self.server.ssh_key))
            .arg("-p")
            .arg(self.server.port.to_string())
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new")
            .arg(self.server.destination())
            .arg(remote_command)
            .stdin(Stdio::null())
            .output()
            .await
            .with_context(|| format!("failed to run remote command: {command}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            bail!("remote command failed: `{command}`\n{stderr}");
        }

        String::from_utf8(output.stdout)
            .map(|stdout| stdout.trim().to_string())
            .context("remote command output was not valid UTF-8")
    }

    pub async fn detect_distro(&self) -> Result<String> {
        let os_release = self.run("cat /etc/os-release").await?;
        let distro = parse_os_release(&os_release);

        if distro.is_empty() {
            bail!("remote distro detection returned an empty result");
        }

        Ok(distro)
    }

    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let command = format!(
            "cat <<'EZVPS' | sudo tee {} >/dev/null\n{}\nEZVPS",
            path, content
        );

        self.run(&command).await.map(|_| ())
    }
}

pub fn validate_local_ssh_key(server: &Server) -> Result<()> {
    let key_path = resolve_ssh_key_path(&server.ssh_key);

    if server.ssh_key.trim().is_empty() {
        bail!(
            "SSH key path is empty.\nSet `ssh_key` to the private key on the machine running ez-vps, for example: {}",
            example_private_key_path()
        );
    }

    if !key_path.exists() {
        bail!(
            "SSH key file was not found: {}\nMake sure `ssh_key` points to a local private key on the machine running ez-vps, for example: {}",
            key_path.display(),
            example_private_key_path()
        );
    }

    Ok(())
}

fn parse_os_release(contents: &str) -> String {
    let mut id = None;
    let mut version_id = None;
    let mut pretty_name = None;

    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = Some(unquote_os_release_value(value));
        } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
            version_id = Some(unquote_os_release_value(value));
        } else if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            pretty_name = Some(unquote_os_release_value(value));
        }
    }

    match (id, version_id, pretty_name) {
        (Some(id), Some(version_id), _) => format!("{id} {version_id}"),
        (_, _, Some(pretty_name)) => pretty_name,
        _ => String::new(),
    }
}

fn unquote_os_release_value(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn resolve_ssh_key_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }

    PathBuf::from(trimmed)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

fn example_private_key_path() -> String {
    let Some(home) = home_dir() else {
        return "~/.ssh/id_ed25519".into();
    };

    home.join(".ssh").join("id_ed25519").display().to_string()
}

async fn diagnose_connection_error(server: &Server, error: &anyhow::Error) -> String {
    let key_path = resolve_ssh_key_path(&server.ssh_key);
    let error_text = error.to_string();

    if error_text.contains("Permission denied (publickey,password)") {
        if is_likely_self_target(server).await {
            return format!(
                "SSH authentication was rejected. It looks like ez-vps may be trying to SSH back into the same machine before your new key is authorized.\n\nTry this on the target machine:\n  mkdir -p ~/.ssh\n  chmod 700 ~/.ssh\n  cat {pub_key} >> ~/.ssh/authorized_keys\n  chmod 600 ~/.ssh/authorized_keys\n\nThen verify it manually:\n  ssh -i {key} {destination}",
                pub_key = public_key_path(&key_path).display(),
                key = key_path.display(),
                destination = server.destination(),
            );
        }

        return format!(
            "SSH authentication was rejected.\n\nCheck these items:\n  - The public key for {} is installed in ~/.ssh/authorized_keys on the target server\n  - The SSH user `{}` is correct\n  - The server allows key-based login on port {}\n\nThen verify it manually:\n  ssh -i {} {}",
            key_path.display(),
            server.user,
            server.port,
            key_path.display(),
            server.destination(),
        );
    }

    if error_text.contains("No such file or directory") && !Path::new(&key_path).exists() {
        return format!(
            "The local SSH key file could not be found.\n\nUpdate `ssh_key` so it points to a private key on the machine running ez-vps, for example:\n  {}",
            example_private_key_path()
        );
    }

    format!(
        "SSH connection failed.\n\nCheck these items:\n  - Host `{}` resolves to the correct server\n  - SSH user `{}` and port `{}` are correct\n  - The private key exists locally at {}\n\nManual test:\n  ssh -i {} -p {} {}",
        server.host,
        server.user,
        server.port,
        key_path.display(),
        key_path.display(),
        server.port,
        server.destination(),
    )
}

fn public_key_path(private_key_path: &Path) -> PathBuf {
    let file_name = private_key_path
        .file_name()
        .map(|name| format!("{}.pub", name.to_string_lossy()))
        .unwrap_or_else(|| "id_ed25519.pub".into());

    private_key_path.with_file_name(file_name)
}

async fn is_likely_self_target(server: &Server) -> bool {
    if matches!(server.host.as_str(), "localhost" | "127.0.0.1" | "::1") {
        return true;
    }

    if let Ok(hostname) = local_hostname().await {
        let hostname = hostname.trim();
        if !hostname.is_empty() && server.host.eq_ignore_ascii_case(hostname) {
            return true;
        }
    }

    if let Ok(targets) = format!("{}:{}", server.host, server.port).to_socket_addrs() {
        let target_ips: Vec<String> = targets.map(|addr| addr.ip().to_string()).collect();
        if !target_ips.is_empty() {
            if let Ok(local_ips) = local_ip_addresses().await {
                return target_ips
                    .iter()
                    .any(|target_ip| local_ips.iter().any(|local_ip| local_ip == target_ip));
            }
        }
    }

    false
}

async fn local_hostname() -> Result<String> {
    let output = Command::new("hostname")
        .stdin(Stdio::null())
        .output()
        .await
        .context("failed to inspect local hostname")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn local_ip_addresses() -> Result<Vec<String>> {
    let command = if cfg!(windows) {
        ("ipconfig", vec![])
    } else {
        ("sh", vec!["-lc", "hostname -I 2>/dev/null || true"])
    };

    let output = Command::new(command.0)
        .args(command.1)
        .stdin(Stdio::null())
        .output()
        .await
        .context("failed to inspect local IP addresses")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if cfg!(windows) {
        Ok(stdout
            .lines()
            .filter_map(|line| line.split(':').nth(1))
            .map(str::trim)
            .filter(|value| value.parse::<std::net::IpAddr>().is_ok())
            .map(ToOwned::to_owned)
            .collect())
    } else {
        Ok(stdout
            .split_whitespace()
            .filter(|value| value.parse::<std::net::IpAddr>().is_ok())
            .map(ToOwned::to_owned)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_os_release, public_key_path};
    use std::path::Path;

    #[test]
    fn parses_os_release_id_and_version() {
        let distro = parse_os_release("ID=ubuntu\nVERSION_ID=\"24.04\"\n");
        assert_eq!(distro, "ubuntu 24.04");
    }

    #[test]
    fn derives_public_key_path_from_private_key() {
        let public = public_key_path(Path::new("/home/demo/.ssh/id_ed25519"));
        assert_eq!(public, Path::new("/home/demo/.ssh/id_ed25519.pub"));
    }
}
