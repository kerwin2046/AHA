#!/bin/bash
# ah-kitty.sh — Kitty selection capture for ah

set -euo pipefail

SELECTION=$(kitty @ get-text --match=selection 2>/dev/null || true)

if [ -z "$SELECTION" ]; then
    SELECTION=$(kitty @ get-text --match=clipboard 2>/dev/null || true)
fi

if [ -z "$SELECTION" ]; then
    notify-send "ah" "No text selected" 2>/dev/null || echo "No text selected"
    exit 1
fi

echo "$SELECTION" | ah explain --pipe | kitty @ new-window --title "ah explain" --hold --stdin false
