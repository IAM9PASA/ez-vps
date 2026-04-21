# ez-vps

`ez-vps` is a Rust CLI for setting up and managing a small VPS over SSH.

It is designed for the common solo-builder workflow:

- initialize a server
- install Docker
- install Caddy or Nginx
- configure a basic firewall
- add and remove app proxy mappings like `api.example.com -> 3000`
- run health checks against the box

## Current commands

### `ez-vps init`

Interactive setup for:

- base packages
- optional Docker
- Caddy or Nginx
- basic firewall rules

Use dry run to preview changes:

```bash
ez-vps init --server prod-1 --dry-run
```

### `ez-vps app add`

Add a new reverse proxy mapping and apply it on the server.

```bash
ez-vps app add --server prod-1 --domain api.example.com --upstream-port 3000 --proxy caddy
```

### `ez-vps app list`

Show saved app mappings for a server.

```bash
ez-vps app list --server prod-1
```

### `ez-vps app remove`

Remove a proxy mapping and re-apply the proxy config.

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

[[servers.apps]]
domain = "api.example.com"
upstream_port = 3000
proxy = "caddy"
```

`servers.toml` is ignored by git so you do not accidentally commit real server details.

## Install

### Quick install

```bash
curl -fsSL https://raw.githubusercontent.com/<your-user>/ez-vps/main/install.sh | REPO_SLUG=<your-user>/ez-vps bash
```

The installer now works like this:

- tries to download a prebuilt GitHub release binary first
- falls back to building from source if no release asset is available

Expected release asset naming:

```txt
ez-vps-x86_64-unknown-linux-gnu.tar.gz
ez-vps-aarch64-unknown-linux-gnu.tar.gz
ez-vps-x86_64-apple-darwin.tar.gz
ez-vps-aarch64-apple-darwin.tar.gz
```

Until the repo is on GitHub, or until you publish release assets, you can run the installer locally:

```bash
bash install.sh
```

Or install from the GitHub repo source directly:

```bash
curl -fsSL https://raw.githubusercontent.com/<your-user>/ez-vps/main/install.sh | REPO_SLUG=<your-user>/ez-vps bash
```

### Manual install

```bash
cargo build --release
./target/release/ez-vps --help
```

## Notes

- The current implementation assumes Ubuntu-style package management with `apt`.
- `deploy` and `status` are still scaffolded and not feature-complete yet.
- A local `ssh` client must be available on the machine running `ez-vps`.

## Publish To GitHub

This directory is already a git repo. To connect it to GitHub:

```bash
git add .
git commit -m "Initial ez-vps scaffold"
git branch -M main
git remote add origin git@github.com:<your-user>/ez-vps.git
git push -u origin main
```

To make the installer use prebuilt binaries, publish GitHub release assets with the filenames listed above.

## Git Order

Yes, you should make the initial commit before expecting GitHub Actions to do anything useful.

Recommended order:

```bash
git add .
git commit -m "Initial ez-vps scaffold"
git branch -M main
git remote add origin git@github.com:<your-user>/ez-vps.git
git push -u origin main
```

Then create a release tag to trigger the workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That tag will trigger `.github/workflows/release.yml` and upload release archives for the installer to use.
