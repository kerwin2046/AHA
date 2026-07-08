#!/usr/bin/env bash
# Optionally bind a global hotkey for ah grab (does not start daemon).
set -euo pipefail

BIN_PATH="${1:-${HOME}/.local/bin/ah}"

info()  { printf "\033[32m%s\033[0m\n" "$*"; }
warn()  { printf "\033[33m%s\033[0m\n" "$*"; }

if [[ ! -x "$BIN_PATH" ]] && ! command -v ah >/dev/null 2>&1; then
    warn "找不到 ah，请先运行 ./start.sh 或 cargo install --path ."
    exit 1
fi

AH="$(command -v ah 2>/dev/null || echo "$BIN_PATH")"
CMD="${AH} grab --quiet --source primary"

setup_gnome() {
    local base="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"
    local path="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/ah/"

    gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "['${path}']"
    gsettings set "${base}:${path}" name 'ah explain selection'
    gsettings set "${base}:${path}" command "$CMD"
    gsettings set "${base}:${path}" binding '<Primary>e'

    info "GNOME 快捷键已绑定：Ctrl+E → 解释当前选中文字"
    info "用法：鼠标选中 → 按 Ctrl+E → 弹出通知（不占用剪贴板）"
}

if [[ "${XDG_CURRENT_DESKTOP:-}" == *"GNOME"* ]] && command -v gsettings >/dev/null 2>&1; then
    setup_gnome
    exit 0
fi

warn "未检测到 GNOME，请手动绑定全局快捷键："
echo
echo "  命令: ${CMD}"
echo "  快捷键: Ctrl+E"
echo
echo "各桌面环境参考 README.md「全局快捷键」一节。"
