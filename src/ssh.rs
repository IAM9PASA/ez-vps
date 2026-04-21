use std::process::Stdio;

use anyhow::{Context, Result, bail};
use tokio::process::Command;

use crate::config::Server;

pub struct SshClient {
    server: Server,
}

impl SshClient {
    pub async fn connect(server: &Server) -> Result<Self> {
        let client = Self {
            server: server.clone(),
        };

        client.run("echo connected").await.with_context(|| {
            format!(
                "failed to connect to {} using local ssh client",
                server.destination()
            )
        })?;

        Ok(client)
    }

    pub async fn run(&self, command: &str) -> Result<String> {
        let remote_command = format!("sh -lc {}", shell_quote(command));
        let output = Command::new("ssh")
            .arg("-i")
            .arg(&self.server.ssh_key)
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
