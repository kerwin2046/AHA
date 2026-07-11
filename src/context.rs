use std::process::Command;

/// Context about the user's current active application.
#[derive(Debug, Default, Clone)]
pub struct ActiveContext {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub language: Option<String>,
    pub file_path: Option<String>,
}

/// Try to detect what the user is currently working on.
pub fn detect_context() -> ActiveContext {
    let mut ctx = ActiveContext::default();

    #[cfg(target_os = "macos")]
    {
        ctx.app_name = get_frontmost_app();
        if let Some(ref app) = ctx.app_name {
            ctx.window_title = get_window_title(app);
            // Try to extract file path / language from window title
            if let Some(ref title) = ctx.window_title {
                ctx.file_path = extract_file_path(title);
                ctx.language = detect_language_from_title(title, app);
            }
        }
    }

    ctx
}

#[cfg(target_os = "macos")]
fn get_frontmost_app() -> Option<String> {
    let script = r#"tell application "System Events"
        get name of first process whose frontmost is true
    end tell"#;
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            Some(s)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn get_window_title(app: &str) -> Option<String> {
    // Only try for known apps that show file paths in window titles
    let title_apps = [
        "Code",
        "Cursor",
        "Windsurf",
        "VSCodium",
        "Zed",
        "Sublime Text",
        "IntelliJ IDEA",
        "CLion",
        "PyCharm",
        "GoLand",
        "WebStorm",
        "RubyMine",
        "Android Studio",
        "Xcode",
        "TextEdit",
        "Terminal",
        "iTerm2",
        "Warp",
        "Ghostty",
        "Kitty",
        "tmux",
    ];

    if !title_apps.iter().any(|a| app.contains(a)) {
        return None;
    }

    let script = format!(
        r#"tell application "System Events"
            tell process "{}"
                get name of front window
            end tell
        end tell"#,
        app
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            Some(s)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn extract_file_path(title: &str) -> Option<String> {
    // Common patterns in window titles:
    //   "main.rs — /Users/me/project/src"  (Zed)
    //   "main.rs - project/src"             (VS Code)
    //   "main.rs (~/project)"               (Sublime)
    //   "bash — 80×24"                      (Terminal)
    //   "~/project — -zsh"                  (Warp)
    //   "README.md - ah"                    (VS Code with folder name)

    // Try to find a path-like pattern
    let parts: Vec<&str> = title.split(" — ").collect();
    if parts.len() >= 2 {
        let candidate = parts[1].trim();
        if candidate.contains('/') || candidate.contains('~') {
            return Some(candidate.to_string());
        }
    }

    // Try " - " separator (VS Code style)
    let parts: Vec<&str> = title.split(" - ").collect();
    if parts.len() >= 2 {
        let candidate = parts[parts.len() - 1].trim();
        if candidate.contains('/') || candidate.contains('~') {
            return Some(candidate.to_string());
        }
        // The filename itself might be in the first part
        if parts[0].contains('.') && parts[0].contains(char::is_alphanumeric) {
            return Some(parts[0].trim().to_string());
        }
    }

    // Check if title itself looks like a file path
    if title.contains('/') || title.contains('~') {
        return Some(title.to_string());
    }

    None
}

#[cfg(target_os = "macos")]
fn detect_language_from_title(title: &str, app: &str) -> Option<String> {
    // First try to find a file extension in the title
    if let Some(ext) = extract_extension(title) {
        if let Some(lang) = extension_to_language(&ext) {
            return Some(lang.to_string());
        }
    }

    // Fall back to app-based detection
    Some(
        match app {
            a if a.contains("Xcode") => "Swift",
            a if a.contains("IntelliJ")
                || a.contains("CLion")
                || a.contains("PyCharm")
                || a.contains("GoLand")
                || a.contains("WebStorm")
                || a.contains("RubyMine") =>
            {
                // These IDEs typically show the file extension in the title
                // We already tried extension extraction above
                return None;
            }
            a if a.contains("Terminal")
                || a.contains("iTerm2")
                || a.contains("Warp")
                || a.contains("Ghostty")
                || a.contains("Kitty") =>
            {
                return None; // Terminal could be anything
            }
            _ => return None,
        }
        .to_string(),
    )
}

#[cfg(target_os = "macos")]
fn extract_extension(title: &str) -> Option<String> {
    // Find patterns like "main.rs", "App.tsx", "Dockerfile" in the title
    for word in title.split(&[' ', '—', '-', '/', '\\', '(', ')', '[', ']', '"', '\''][..]) {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        if let Some(dot) = word.rfind('.') {
            let ext = word[dot + 1..].to_lowercase();
            if ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphabetic()) {
                return Some(ext);
            }
        }
        // Check for extension-less files like "Dockerfile", "Makefile"
        if word == "Dockerfile" || word == "Makefile" || word == "CMakeLists" {
            return Some(word.to_lowercase());
        }
    }
    None
}

fn extension_to_language(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "Rust",
        "ts" => "TypeScript",
        "tsx" => "TypeScript React",
        "js" => "JavaScript",
        "jsx" => "JavaScript React",
        "py" => "Python",
        "go" => "Go",
        "java" => "Java",
        "rb" => "Ruby",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "cs" => "C#",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "scala" => "Scala",
        "zig" => "Zig",
        "sh" | "bash" => "Bash",
        "zsh" => "Zsh",
        "lua" => "Lua",
        "pl" => "Perl",
        "php" => "PHP",
        "r" => "R",
        "sql" => "SQL",
        "yaml" | "yml" => "YAML",
        "json" => "JSON",
        "toml" => "TOML",
        "md" => "Markdown",
        "html" => "HTML",
        "css" => "CSS",
        "dart" => "Dart",
        "ex" | "exs" => "Elixir",
        "clj" | "cljs" => "Clojure",
        "hs" => "Haskell",
        "dockerfile" => "Dockerfile",
        "makefile" => "Makefile",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_extract_extension_rust() {
        let ext = extract_extension("main.rs — /Users/me/project");
        assert_eq!(ext, Some("rs".to_string()));
    }

    #[test]
    fn test_extract_extension_tsx() {
        let ext = extract_extension("App.tsx - my-app");
        assert_eq!(ext, Some("tsx".to_string()));
    }

    #[test]
    fn test_extract_extension_dockerfile() {
        let ext = extract_extension("Dockerfile - project");
        assert_eq!(ext, Some("dockerfile".to_string()));
    }

    #[test]
    fn test_extension_to_language() {
        assert_eq!(extension_to_language("rs"), Some("Rust"));
        assert_eq!(extension_to_language("py"), Some("Python"));
        assert_eq!(extension_to_language("tsx"), Some("TypeScript React"));
        assert_eq!(extension_to_language("dockerfile"), Some("Dockerfile"));
        assert_eq!(extension_to_language("unknown"), None);
    }
}
