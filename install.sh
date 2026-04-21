#!/usr/bin/env bash
set -euo pipefail

PACKAGE_NAME="ez-vps"
REPO_SLUG="${REPO_SLUG:-}"
REPO_URL="${REPO_URL:-}"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=""

cleanup() {
  if [[ -n "${TMP_DIR}" && -d "${TMP_DIR}" ]]; then
    rm -rf "${TMP_DIR}"
  fi
}

trap cleanup EXIT

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

say() {
  echo "==> $*"
}

detect_profile() {
  if [[ -n "${PROFILE_FILE:-}" ]]; then
    echo "${PROFILE_FILE}"
    return 0
  fi

  if [[ -n "${ZSH_VERSION:-}" ]]; then
    echo "${HOME}/.zshrc"
    return 0
  fi

  if [[ -f "${HOME}/.bashrc" ]]; then
    echo "${HOME}/.bashrc"
    return 0
  fi

  if [[ -f "${HOME}/.profile" ]]; then
    echo "${HOME}/.profile"
    return 0
  fi

  echo "${HOME}/.bashrc"
}

ensure_path() {
  case ":$PATH:" in
    *":${INSTALL_DIR}:"*)
      return 0
      ;;
  esac

  local profile_file export_line
  profile_file="$(detect_profile)"
  export_line="export PATH=\"${INSTALL_DIR}:\$PATH\""

  mkdir -p "$(dirname "${profile_file}")"
  touch "${profile_file}"

  if ! grep -Fqx "${export_line}" "${profile_file}"; then
    printf '\n%s\n' "${export_line}" >> "${profile_file}"
    say "Added ${INSTALL_DIR} to PATH in ${profile_file}"
  else
    say "${INSTALL_DIR} is already configured in ${profile_file}"
  fi

  echo "Run this to use ez-vps in the current shell:"
  echo "source \"${profile_file}\""
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Linux) os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    *)
      echo "Unsupported OS: ${os}" >&2
      return 1
      ;;
  esac

  case "${arch}" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)
      echo "Unsupported architecture: ${arch}" >&2
      return 1
      ;;
  esac

  echo "${arch}-${os}"
}

release_asset_name() {
  local target="$1"
  echo "${PACKAGE_NAME}-${target}.tar.gz"
}

release_download_url() {
  local asset_name="$1"

  if [[ "${VERSION}" == "latest" ]]; then
    echo "https://github.com/${REPO_SLUG}/releases/latest/download/${asset_name}"
  else
    echo "https://github.com/${REPO_SLUG}/releases/download/${VERSION}/${asset_name}"
  fi
}

install_binary() {
  local binary_path="$1"

  mkdir -p "${INSTALL_DIR}"
  cp "${binary_path}" "${INSTALL_DIR}/${PACKAGE_NAME}"
  chmod +x "${INSTALL_DIR}/${PACKAGE_NAME}"

  say "Installed to ${INSTALL_DIR}/${PACKAGE_NAME}"
  ensure_path

  say "Done"
  echo "Run: ${PACKAGE_NAME} --help"
}

install_from_release() {
  local target asset_name url archive_path extract_dir

  if [[ -z "${REPO_SLUG}" ]]; then
    return 1
  fi

  need_cmd curl || {
    echo "curl is required to download a release binary." >&2
    return 1
  }

  need_cmd tar || {
    echo "tar is required to unpack a release binary." >&2
    return 1
  }

  target="$(detect_target)" || return 1
  asset_name="$(release_asset_name "${target}")"
  url="$(release_download_url "${asset_name}")"

  TMP_DIR="$(mktemp -d)"
  archive_path="${TMP_DIR}/${asset_name}"
  extract_dir="${TMP_DIR}/extract"
  mkdir -p "${extract_dir}"

  say "Downloading release asset ${asset_name}"
  if ! curl -fsSL "${url}" -o "${archive_path}"; then
    echo "Release download not available at ${url}" >&2
    return 1
  fi

  tar -xzf "${archive_path}" -C "${extract_dir}"

  if [[ -x "${extract_dir}/${PACKAGE_NAME}" ]]; then
    install_binary "${extract_dir}/${PACKAGE_NAME}"
    return 0
  fi

  if [[ -x "${extract_dir}/target/release/${PACKAGE_NAME}" ]]; then
    install_binary "${extract_dir}/target/release/${PACKAGE_NAME}"
    return 0
  fi

  echo "Downloaded archive did not contain ${PACKAGE_NAME}" >&2
  return 1
}

install_from_source() {
  local project_dir

  if ! need_cmd cargo; then
    echo "Rust and cargo are required for source installation." >&2
    echo "Install Rust from https://rustup.rs and run this script again." >&2
    exit 1
  fi

  if [[ -f "Cargo.toml" ]] && grep -q '^name = "ez-vps"' "Cargo.toml"; then
    project_dir="$(pwd)"
    say "Using current directory: ${project_dir}"
  else
    if [[ -z "${REPO_URL}" ]]; then
      if [[ -n "${REPO_SLUG}" ]]; then
        REPO_URL="https://github.com/${REPO_SLUG}.git"
      else
        echo "REPO_URL is not set." >&2
        echo "Run this inside the ez-vps repo or set REPO_SLUG/REPO_URL." >&2
        exit 1
      fi
    fi

    if ! need_cmd git; then
      echo "git is required to clone the repository." >&2
      exit 1
    fi

    TMP_DIR="$(mktemp -d)"
    project_dir="${TMP_DIR}/ez-vps"
    say "Cloning ${REPO_URL}"
    git clone "${REPO_URL}" "${project_dir}"
  fi

  say "Building release binary from source"
  cargo build --release --manifest-path "${project_dir}/Cargo.toml"
  install_binary "${project_dir}/target/release/${PACKAGE_NAME}"
}

say "Installing ${PACKAGE_NAME}"

if install_from_release; then
  exit 0
fi

say "Falling back to source install"
install_from_source
