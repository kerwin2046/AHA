use anyhow::{Context, Result};
use std::process::Command;

/// Where to read text from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Try PRIMARY selection first, fall back to the clipboard.
    Auto,
    /// Only the PRIMARY selection (mouse-highlighted text, no copy needed).
    Primary,
    /// Only the system clipboard (Ctrl+C content).
    Clipboard,
}

impl Source {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Source::Auto),
            "primary" => Ok(Source::Primary),
            "clipboard" | "clip" => Ok(Source::Clipboard),
            other => anyhow::bail!("Unknown source '{other}' (use: auto, primary, clipboard)"),
        }
    }
}

/// Normalize line endings and surrounding whitespace from a selection.
pub fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

struct Backend {
    program: &'static str,
    args: &'static [&'static str],
}

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

fn primary_backends() -> Vec<Backend> {
    if is_wayland() {
        vec![
            Backend {
                program: "wl-paste",
                args: &["--primary"],
            },
            Backend {
                program: "xclip",
                args: &["-o", "-selection", "primary"],
            },
            Backend {
                program: "xsel",
                args: &["-p"],
            },
        ]
    } else {
        vec![
            Backend {
                program: "xclip",
                args: &["-o", "-selection", "primary"],
            },
            Backend {
                program: "xsel",
                args: &["-p"],
            },
            Backend {
                program: "wl-paste",
                args: &["--primary"],
            },
        ]
    }
}

fn clipboard_backends() -> Vec<Backend> {
    if is_wayland() {
        vec![
            Backend {
                program: "wl-paste",
                args: &[],
            },
            Backend {
                program: "xclip",
                args: &["-o", "-selection", "clipboard"],
            },
            Backend {
                program: "xsel",
                args: &["-b"],
            },
        ]
    } else {
        vec![
            Backend {
                program: "xclip",
                args: &["-o", "-selection", "clipboard"],
            },
            Backend {
                program: "xsel",
                args: &["-b"],
            },
            Backend {
                program: "wl-paste",
                args: &[],
            },
        ]
    }
}

fn run_backend(b: &Backend) -> Option<String> {
    let output = Command::new(b.program).args(b.args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = normalize(&String::from_utf8_lossy(&output.stdout));
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn read_from(backends: &[Backend]) -> Option<String> {
    backends.iter().find_map(run_backend)
}

fn read_arboard() -> Option<String> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let text = normalize(&cb.get_text().ok()?);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Read the system clipboard (for daemon / copy-triggered use).
pub fn read_clipboard() -> Option<String> {
    read_from(&clipboard_backends()).or_else(read_arboard)
}

/// Read the selected text according to the chosen source, with fallbacks.
pub fn read_selection(source: Source) -> Result<String> {
    let text = match source {
        Source::Primary => read_from(&primary_backends()),
        Source::Clipboard => read_clipboard(),
        Source::Auto => read_from(&primary_backends()).or_else(read_clipboard),
    };

    text.with_context(|| {
        "No selected text found. Highlight some text first.\n\
         If this keeps happening, install a helper for your session:\n  \
         Wayland: wl-clipboard   (provides wl-paste)\n  \
         X11:     xclip  or  xsel"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_multiline() {
        assert_eq!(normalize("a\nb"), "a\nb");
        assert_eq!(normalize("a\r\nb\r\n"), "a\nb");
        assert_eq!(normalize("  line1\nline2  "), "line1\nline2");
    }

    #[test]
    fn test_source_parse() {
        assert_eq!(Source::parse("auto").unwrap(), Source::Auto);
        assert_eq!(Source::parse("PRIMARY").unwrap(), Source::Primary);
        assert_eq!(Source::parse("clip").unwrap(), Source::Clipboard);
        assert!(Source::parse("bogus").is_err());
    }
}
