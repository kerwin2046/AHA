#!/usr/bin/env bash
set -euo pipefail

REPO="USER/ah"
BIN_NAME="ah"
INSTALL_DIR="${HOME}/.local/bin"
BIN_PATH="${INSTALL_DIR}/${BIN_NAME}"
CONFIG_DIR="${HOME}/.config/ah/"
DATA_DIR="${HOME}/.local/share/ah/"

# --- Utilities -------------------------------------------------------

info()  { printf "\033[32m%s\033[0m\n" "$*" >&2; }
warn()  { printf "\033[33m%s\033[0m\n" "$*" >&2; }
error() { printf "\033[31m%s\033[0m\n" "$*" >&2; exit 1; }

cleanup() {
    [ -n "${TMPDIR:-}" ] && [ -d "$TMPDIR" ] && rm -rf "$TMPDIR"
}
trap cleanup EXIT

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      error "Unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}_${arch}"
}

# --- Latest Release --------------------------------------------------

get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local version

    if command -v curl >/dev/null 2>&1; then
        version=$(curl -sL "$api_url" | grep '"tag_name"' | cut -d'"' -f4)
    elif command -v wget >/dev/null 2>&1; then
        version=$(wget -qO- "$api_url" | grep '"tag_name"' | cut -d'"' -f4)
    else
        error "Neither curl nor wget found. Install one of them and retry."
    fi

    [ -z "$version" ] && error "Could not determine latest version from GitHub API."
    echo "$version"
}

# --- Download & Install ----------------------------------------------

download_and_install() {
    local version="$1"
    local platform="$2"
    local os arch
    os="${platform%_*}"
    arch="${platform#*_}"
    local archive_url="https://github.com/${REPO}/releases/download/${version}/ah-${os}-${arch}.tar.gz"
    local archive_name="ah-${os}-${arch}.tar.gz"

    TMPDIR="$(mktemp -d)"
    cd "$TMPDIR"

    info "Downloading ${archive_url} ..."
    if command -v curl >/dev/null 2>&1; then
        curl -sL -o "$archive_name" "$archive_url"
    else
        wget -qO "$archive_name" "$archive_url"
    fi

    if [ ! -f "$archive_name" ] || [ ! -s "$archive_name" ]; then
        return 1
    fi

    info "Extracting ..."
    tar xzf "$archive_name"

    if [ ! -f "${BIN_NAME}" ]; then
        return 1
    fi

    mkdir -p "$INSTALL_DIR"
    mv "${BIN_NAME}" "$BIN_PATH"
    chmod +x "$BIN_PATH"

    info "Installed ${BIN_NAME} to ${BIN_PATH}"
    return 0
}

# --- Build from Source (fallback) ------------------------------------

build_from_source() {
    info "Download prebuilt binary failed. Attempting to build from source ..."

    if ! command -v cargo >/dev/null 2>&1; then
        error "cargo is required to build from source. Install Rust: https://rustup.rs"
    fi

    local tmp_src
    tmp_src="$(mktemp -d)"
    cd "$tmp_src"

    info "Cloning ${REPO} ..."
    git clone "https://github.com/${REPO}.git" .
    cargo build --release

    mkdir -p "$INSTALL_DIR"
    cp "target/release/${BIN_NAME}" "$BIN_PATH"
    chmod +x "$BIN_PATH"

    cd /
    rm -rf "$tmp_src"

    info "Built and installed ${BIN_NAME} to ${BIN_PATH}"
}

# --- Init Configuration ----------------------------------------------

run_init() {
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
        info "Running '${BIN_NAME} init' to generate config ..."
        "${BIN_PATH}" init
    fi
}

# --- Main -----------------------------------------------------------

main() {
    info "ah — install script"
    echo

    local platform
    platform="$(detect_platform)"
    info "Detected platform: ${platform}"

    # Check existing installation
    if [ -f "$BIN_PATH" ]; then
        local current_version
        current_version="$("${BIN_PATH}" --version 2>/dev/null || echo "unknown")"
        # Use /dev/tty if available so the prompt works under curl | bash
        local reply="y"
        if [ -t 0 ]; then
            read -r -p "Upgrade? [Y/n] " reply
        fi
        case "${reply:-y}" in
            [Yy]*|[Yy][Ee][Ss]*) ;;
            *) info "Skipping upgrade."; exit 0 ;;
        esac
    fi

    local version
    version="$(get_latest_version)"
    info "Latest release: ${version}"

    if download_and_install "$version" "$platform"; then
        run_init
        info "${BIN_NAME} ${version} installed successfully!"
        info "You may need to add '${INSTALL_DIR}' to your PATH."
        info "Try: ${BIN_NAME} --help"
    else
        build_from_source
        run_init
        info "${BIN_NAME} installed via cargo."
        info "Try: ${BIN_NAME} --help"
    fi
}

main "$@"
