#!/usr/bin/env bash
# Integration test: daemon single-instance + one explain per clipboard copy.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ROOT}/target/release/ah"
LOG="/tmp/ah-daemon-test.log"
TEST_DIR="/tmp/ah-test-$$"
export XDG_CONFIG_HOME="${TEST_DIR}/config"
export XDG_DATA_HOME="${TEST_DIR}/data"

pass() { printf "\033[32m✓ %s\033[0m\n" "$*"; }
fail() { printf "\033[31m✗ %s\033[0m\n" "$*"; exit 1; }
info() { printf "\033[36m→ %s\033[0m\n" "$*"; }

cleanup() {
    pkill -f "${BIN} daemon" 2>/dev/null || true
    pkill -f 'ah daemon' 2>/dev/null || true
    rm -rf "$TEST_DIR" 2>/dev/null || true
}
trap cleanup EXIT

set_clipboard() {
    local text="$1"
    if command -v wl-copy >/dev/null 2>&1; then
        printf '%s' "$text" | wl-copy
    elif command -v xclip >/dev/null 2>&1; then
        printf '%s' "$text" | xclip -selection clipboard
    else
        fail "需要 wl-copy 或 xclip 来模拟剪贴板"
    fi
}

count_triggers() {
    grep -c '→' "$LOG" 2>/dev/null || echo 0
}

mkdir -p "${XDG_CONFIG_HOME}/ah" "${XDG_DATA_HOME}/ah"
cat >"${XDG_CONFIG_HOME}/ah/config.toml" <<'EOF'
[provider]
default = "mock"

[provider.mock]
response = '{"translation":"测试词","explanation":"mock explanation","usage":"mock()"}'
EOF

info "Building release binary..."
(cd "$ROOT" && cargo build --release -q)

info "Running unit tests..."
(cd "$ROOT" && cargo test -q)

info "Cleaning old daemons..."
pkill -f 'ah daemon' 2>/dev/null || true
sleep 0.3

: >"$LOG"
info "Starting daemon (mock provider)..."
nohup "$BIN" daemon >>"$LOG" 2>&1 &
sleep 1

pgrep -f "${BIN} daemon" >/dev/null || fail "daemon 未启动"
DAEMON_COUNT=$(pgrep -fc "${BIN} daemon" || echo 0)
[[ "$DAEMON_COUNT" -eq 1 ]] || fail "应只有 1 个 daemon 进程，实际: ${DAEMON_COUNT}"
pass "单实例启动正常 (1 个进程)"

info "第二次启动应被拒绝..."
if "$BIN" daemon >>"$LOG" 2>&1; then
    fail "第二个 daemon 不应启动成功"
else
    pass "重复启动被正确拒绝"
fi

info "模拟复制 'ah init' 一次..."
set_clipboard "ah init"
sleep 2.5

TRIGGERS=$(count_triggers)
[[ "$TRIGGERS" -eq 1 ]] || {
    echo "--- daemon log ---"
    cat "$LOG"
    fail "复制一次应触发 1 次，实际: ${TRIGGERS}"
}
pass "复制一次只触发 1 次解释"

info "相同内容再复制一次（10s 去重窗口内）..."
set_clipboard "ah init"
sleep 2.5

TRIGGERS2=$(count_triggers)
[[ "$TRIGGERS2" -eq 1 ]] || {
    cat "$LOG"
    fail "去重窗口内重复复制应仍为 1 次，实际: ${TRIGGERS2}"
}
pass "去重窗口内不重复触发"

info "模拟复制多行内容..."
set_clipboard $'pub fn foo() {\n    bar()\n}'
sleep 2.5

TRIGGERS3=$(count_triggers)
[[ "$TRIGGERS3" -eq 2 ]] || {
    cat "$LOG"
    fail "多行复制应再触发 1 次（共 2 次），实际触发行数: ${TRIGGERS3}"
}
pass "多行复制正常触发"

grep -q '测试词' "$LOG" || fail "日志中缺少 mock 翻译输出"
pass "mock 翻译输出正常"

echo
pass "全部集成测试通过"
