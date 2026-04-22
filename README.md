# ez-vps

`ez-vps` is a Rust CLI for bootstrapping and managing a small VPS over SSH.

It focuses on the common solo-builder flow:

- initialize a server
- install Docker
- install Caddy or Nginx
- configure a basic firewall
- add and remove reverse proxy mappings like `api.example.com -> 8000`
- run health checks against the box

## What works now

### `ez-vps init`

Interactive server setup for:

- base packages
- optional Docker
- Caddy or Nginx
- firewall rules

Preview changes first:

```bash
ez-vps init --server prod-1 --dry-run
```

### `ez-vps update`

Refresh apt packages on the server and re-apply the saved proxy config.

```bash
ez-vps update --server prod-1
```

### `ez-vps app add`

Add a reverse proxy mapping and apply it on the server.

```bash
ez-vps app add --server prod-1 --domain api.example.com --upstream-port 8000 --proxy caddy
```

### `ez-vps app list`

List saved app mappings for a server.

```bash
ez-vps app list --server prod-1
```

### `ez-vps app remove`

Remove a saved mapping and re-apply the proxy config.

```bash
ez-vps app remove --server prod-1 --domain api.example.com
```

### `ez-vps check`

Verify:

- SSH connectivity
- distro detection
- Docker installation
- proxy installation
- firewall status
- saved app reachability

```bash
ez-vps check --server prod-1
```

### `ez-vps uninstall`

Remove ez-vps-managed proxy services from a server. This command asks for confirmation before doing anything destructive.

```bash
ez-vps uninstall --server prod-1
```

## Current limitations

- Ubuntu-style `apt` setup is the main supported path right now.
- `deploy` and `status` are still scaffolded and not feature-complete.
- `update` and `uninstall` act on services tracked in `servers.toml`.
- A local `ssh` client must be available on the machine running `ez-vps`.
- Mixing Caddy and Nginx app mappings on the same server is not supported yet.

## Config

By default the CLI reads `servers.toml`.

Start from the example file:

```bash
cp servers.example.toml servers.toml
```

Example:

```toml
[[servers]]
name = "prod-1"
host = "1.2.3.4"
user = "root"
port = 22
ssh_key = "/home/you/.ssh/id_ed25519"
managed_docker = true
managed_proxy = "caddy"

[[servers.apps]]
domain = "api.example.com"
upstream_port = 8000
proxy = "caddy"
```

`servers.toml` is gitignored so real server details do not get committed by accident.

## Install

### Quick install

If the repository is public and release assets are published:

```bash
curl -fsSL https://raw.githubusercontent.com/IAM9PASA/ez-vps/main/install.sh | REPO_SLUG=IAM9PASA/ez-vps bash
```

The installer:

- tries to download a prebuilt GitHub release binary first
- falls back to building from source if no release asset is available
- adds `~/.local/bin` to your shell profile if needed

If `ez-vps` is still not found right after install, open a new terminal or run:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

During `ez-vps init`, the interactive flow now suggests:

- your current local SSH user
- a likely local private key path such as `~/.ssh/id_ed25519`
- app port `8000` by default when no existing app mapping is saved yet

Expected release asset names:

```txt
ez-vps-x86_64-unknown-linux-gnu.tar.gz
ez-vps-aarch64-unknown-linux-gnu.tar.gz
ez-vps-x86_64-apple-darwin.tar.gz
ez-vps-aarch64-apple-darwin.tar.gz
```

If the repository is private, public `curl` install will not work without authentication. In that case, clone the repo or run the installer locally.

### Local install

```bash
bash install.sh
```

### Manual install

```bash
cargo build --release
./target/release/ez-vps --help
```

## Troubleshooting

### `Permission denied (publickey,password)`

This usually means the `ssh_key` private key is not authorized on the target machine yet.

First test SSH manually:

```bash
ssh -i ~/.ssh/id_ed25519 your-user@your-server-ip
```

If you are running `ez-vps` on the VPS itself and it is trying to SSH back into the same machine, add the generated public key to `authorized_keys`:

```bash
mkdir -p ~/.ssh
chmod 700 ~/.ssh
cat ~/.ssh/id_ed25519.pub >> ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys
```

Then retry `ez-vps init`.

## GitHub Actions release flow

The release workflow lives in `.github/workflows/release.yml`.

It runs on pushed tags matching `v*`, for example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That workflow builds release archives for Linux and macOS and uploads them to the GitHub release so `install.sh` can use them.

## Development

Common local commands:

```bash
cargo check
cargo run -- --help
cargo run -- init --server prod-1 --dry-run
```
