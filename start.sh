#!/usr/bin/env bash
# One-command local setup: build, install to PATH, start daemon.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
BIN_PATH="${BIN_DIR}/ah"
LOG_DIR="${HOME}/.local/share/ah"
LOG_FILE="${LOG_DIR}/daemon.log"

info()  { printf "\033[32m%s\033[0m\n" "$*"; }
warn()  { printf "\033[33m%s\033[0m\n" "$*"; }

mkdir -p "$BIN_DIR" "$LOG_DIR"

info "Building ah ..."
(cd "$ROOT" && cargo build --release)

ln -sf "${ROOT}/target/release/ah" "$BIN_PATH"
info "Installed: ${BIN_PATH}"

if pgrep -f 'ah daemon' >/dev/null 2>&1; then
    pkill -f 'ah daemon' 2>/dev/null || true
    sleep 0.5
    # Force-kill stragglers (multiple daemons cause duplicate explains).
    if pgrep -f 'ah daemon' >/dev/null 2>&1; then
        pkill -9 -f 'ah daemon' 2>/dev/null || true
        sleep 0.2
    fi
    info "Stopped old daemon(s)."
fi

# Remove stale lock files from older versions.
rm -f "${HOME}/.local/share/ah/daemon.lock" 2>/dev/null || true

nohup "$BIN_PATH" daemon >>"$LOG_FILE" 2>&1 &
sleep 0.2

if pgrep -f 'ah daemon' >/dev/null 2>&1; then
    info "ah daemon started."
    echo
    info "用法：选中文字 → Ctrl+C 复制 → 自动弹出翻译和解释"
    info "日志：tail -f ${LOG_FILE}"
    info "停止：pkill -f 'ah daemon'"
else
    warn "daemon 可能启动失败，查看日志："
    tail -5 "$LOG_FILE" 2>/dev/null || true
    exit 1
fi

if ! command -v ah >/dev/null 2>&1; then
    echo
    warn "~/.local/bin 不在 PATH 里，当前 shell 可运行："
    warn "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    warn "或把上面这行加到 ~/.zshrc"
fi
