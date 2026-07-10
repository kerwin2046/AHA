# ah

> Stay in flow.

**选中即懂，AI 即时解释。**

**[English](README.en.md)** · 中文

翻译术语、解释代码、搞懂 API 和陌生标识符——
不必离开终端。

名字来自一声「啊？」——遇到陌生标识符时，脑子里自然的反应。

<img width="3024" height="1864" alt="image" src="https://github.com/user-attachments/assets/4213c8a5-987d-472f-adb1-76e8ca3aa3b8" />

## 快速开始

```bash
# 从源码构建并启动后台守护（推荐）
./start.sh

# 或手动安装
cargo install --path .

# 首次配置 AI 服务
ah init
```

配置好 API Key 后，日常只需：

```
选中文字 → Ctrl+C 复制 → 自动弹出翻译与解释
```

## 安装

```bash
# 一键安装（Linux / macOS，x86_64 / aarch64）
curl -sSL https://raw.githubusercontent.com/USER/ah/main/install.sh | bash

# Homebrew（macOS）
brew install USER/ah/ah

# 从源码构建
cargo install --path .
```

构建后确保 `~/.local/bin` 在 `PATH` 中：

```bash
export PATH="${HOME}/.local/bin:${PATH}"
```

## 核心概念

`ah` 围绕一个动作设计：**把选中的文字变成可理解的解释**。根据你的习惯，有三种触发方式：

| 方式 | 命令 | 适合场景 |
|------|------|----------|
| 复制触发 | `ah daemon` | 通用，任何应用都适用 |
| 快捷键触发 | `ah grab` | 不想占用剪贴板 |
| 手动查询 | `ah explain` | 脚本、管道、精确控制 |

三种方式共享同一套 AI 后端、过滤规则和历史记录。

---

## 用法

### 复制即解释（推荐）

后台守护进程监听剪贴板，复制后自动解释：

```bash
export TX_DEEPSEEK_KEY="sk-..."   # 或其他 provider
ah daemon &
```

`./start.sh` 会自动完成构建、安装和启动。

守护进程的行为：

- **去抖** 800ms — 连续复制只触发一次
- **去重** — 相同内容不重复解释
- **过滤** — 跳过纯 URL、纯数字、纯标点、不足 2 字符的内容
- **单实例** — 防止多个守护进程同时运行

```bash
tail -f ~/.local/share/ah/daemon.log   # 查看日志
pkill -f 'ah daemon'                   # 停止守护
```

### 解释一个词

```bash
ah explain map
ah explain --expand serialize          # 详细解释
ah explain --json useEffect            # JSON 输出，供脚本使用
ah explain --to English replicate      # 指定翻译目标语言
```

### 管道模式

从 stdin 读取选中内容，适合编辑器集成：

```bash
echo 'Array.prototype' | ah explain --pipe
```

### 带文件上下文

读取源码附近几行，让 AI 结合上下文解释：

```bash
ah explain -f src/main.rs:42
ah explain -f src/providers/ollama.rs:15 -c 10   # 上下各 10 行
```

### 快捷键：选中即解释

`ah grab` 直接读取鼠标选中区，**不修改剪贴板**：

```bash
ah grab                          # 终端输出
ah grab --quiet                  # 仅桌面通知
ah grab --source primary         # 只读 PRIMARY 选区
```

依赖（按桌面环境任选其一）：

| 会话 | 工具包 | 命令 |
|------|--------|------|
| Wayland | `wl-clipboard` | `wl-paste` |
| X11 | `xclip` 或 `xsel` | `xclip` / `xsel` |

**GNOME** — 运行 `./scripts/setup-hotkey.sh` 自动绑定 Ctrl+E，或手动配置：

```bash
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \
  "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/ah/']"
BASE=org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/ah/
gsettings set "$BASE" name 'ah grab'
gsettings set "$BASE" command 'ah grab --quiet --source primary'
gsettings set "$BASE" binding '<Super>e'
```

**Hyprland**：`bind = SUPER, E, exec, ah grab --quiet --source primary`

**Sway**：`bindsym $mod+e exec ah grab --quiet --source primary`

**i3**：`bindsym $mod+e exec --no-startup-id ah grab --quiet --source primary`

### 交互模式

```bash
ah ask
```

进入 REPL，输入词回车即得解释。适合连续查多个词。

### 历史记录

每次查询自动保存，支持回顾和统计：

```bash
ah history                  # 最近 20 条
ah history -n 50            # 最近 50 条
ah history -s serde         # 搜索
ah history --stats          # 统计
ah tui                      # TUI 交互浏览
```

---

## 编辑器与终端集成

`ah` 不绑定特定工具——它通过管道和剪贴板接入你已有的环境。

### Vim / Neovim

```bash
cp -r vim-ah ~/.vim/pack/plugins/start/vim-ah
# vim-plug: Plug '~/path/to/ah/vim-ah'
```

| 操作 | 效果 |
|------|------|
| `<leader>kt` | 解释光标下的词 |
| `:AH map` | 解释指定词 |
| `y` / `yy` | 复制后自动弹出解释（默认开启） |

关闭自动解释：`let g:ah_yank_auto = 0`

若要让 Vim 复制触发 `ah daemon` 的系统通知，需同步到系统剪贴板：

```vim
set clipboard=unnamedplus
```

Neovim 使用浮动窗口，Vim 使用 scratch buffer。

### tmux

在 `~/.tmux.conf` 中添加：

```tmux
bind-key -T copy-mode-vi x send -X copy-pipe-and-cancel \
    "ah explain --pipe | tmux display-popup -w80% -h50% -T 'ah explain'"
```

`prefix + [` → 选中单词 → `x` → 浮窗弹出解释。

也可直接 source 插件：`run ~/path/to/ah/tmux-ah/ah.tmux`

### Kitty

```bash
cp kitty-ah/ah-kitty.sh ~/.local/bin/
```

在 `~/.config/kitty/kitty.conf` 中：

```
map ctrl+e shell -x ah-kitty.sh
```

选中文本 → `Ctrl+E` → 新窗口显示解释。

### WezTerm

在 `~/.config/wezterm/wezterm.lua` 中：

```lua
require 'wezterm-ah'

local keys = {
  { key = 'e', mods = 'SUPER', action = wezterm.action.EmitEvent('ah-explain') },
}
```

选中文本 → `Super+E` → 右侧分屏显示解释。

---

## 配置

配置文件：`~/.config/ah/config.toml`

运行 `ah init` 交互式生成，或手动创建：

```toml
[provider]
default = "auto"    # auto / ollama / openai / deepseek / anthropic

[provider.ollama]
model = "llama3.2"
url = "http://localhost:11434"

[provider.openai]
model = "gpt-4o-mini"
# api_key = "sk-..."    # 或通过环境变量

[provider.deepseek]
model = "deepseek-chat"
url = "https://api.deepseek.com/v1"

[display]
theme = "auto"
```

环境变量优先级高于配置文件：

| 变量 | 用途 |
|------|------|
| `TX_OPENAI_KEY` | OpenAI API Key |
| `TX_DEEPSEEK_KEY` | DeepSeek API Key |
| `TX_ANTHROPIC_KEY` | Anthropic API Key |
| `TX_PROVIDER` | 强制指定 provider |

### AI Provider

| 名称 | 类型 | 默认模型 |
|------|------|----------|
| Ollama | 本地 | `llama3.2` |
| OpenAI | 云端 | `gpt-4o-mini` |
| DeepSeek | 云端 | `deepseek-chat` |
| Anthropic | 云端 | `claude-3-haiku` |

选择逻辑：

1. `--provider` 参数
2. 配置文件 `[provider] default`
3. 自动检测：先尝试本地 Ollama，再检查已配置的云端 API Key

---

## 输出示例

```
────────────────────────────────
翻译: 迭代器

解释: Rust 中用于遍历集合的 trait。
通过 .next() 方法逐个返回元素，
支持 for 循环和各种适配器。

用法:
  for item in vec.iter() {
      println!("{item}");
  }
────────────────────────────────
```

---

## 命令参考

```
ah explain <word>       解释一个词
ah explain --pipe       从 stdin 读取
ah explain -f file:line 带文件上下文
ah daemon               复制即解释（后台）
ah grab                 解释当前选中文字
ah grab --quiet         仅通知，不输出终端
ah ask                  交互模式
ah tui                  历史 TUI 浏览器
ah history              查看历史
ah history --stats      查询统计
ah init                 初始化配置
ah config               查看当前配置
```

---

## 项目结构

```
ah/
├── src/            核心 Rust 实现
├── vim-ah/         Vim / Neovim 插件
├── tmux-ah/        tmux 集成
├── kitty-ah/       Kitty 集成
├── wezterm-ah/     WezTerm 集成
├── scripts/        安装与测试脚本
├── start.sh        一键构建 + 启动守护
└── install.sh      远程安装脚本
```

---

<p align="center">
  <sub>Stay in flow.</sub>
</p>
