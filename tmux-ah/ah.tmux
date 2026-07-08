#!/bin/bash
# ah.tmux — tmux plugin entry point
# Source this from tmux, or add to ~/.tmux.conf:
#   run ~/.local/share/ah/tmux-ah/ah.tmux

CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Register keybinding: in copy-mode, press 'x' to explain selection
tmux bind-key -T copy-mode-vi x send -X copy-pipe-and-cancel \
    "ah explain --pipe | tmux display-popup -w80% -h50% -T 'ah explain'"
