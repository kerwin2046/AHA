use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ah", about = "Terminal translate & explain")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Translate and explain a word/identifier
    Explain {
        /// Word to explain (omit if using --pipe)
        word: Option<String>,

        /// Read word from stdin (pipe mode)
        #[arg(long)]
        pipe: bool,

        /// Read word from file, optionally with line number (e.g. file.rs:42)
        #[arg(short = 'f', long)]
        file: Option<String>,

        /// AI provider to use (ollama, openai, deepseek, anthropic)
        #[arg(short = 'p', long)]
        provider: Option<String>,

        /// Lines of context to capture when using --file
        #[arg(short = 'c', long, default_value = "5")]
        context: usize,

        /// Show detailed explanation
        #[arg(long)]
        expand: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Output format: plain, json, markdown, html
        #[arg(long, default_value = "plain")]
        format: String,

        /// Target language for translation (e.g. 中文, English, 日本語)
        #[arg(long, default_value = "中文")]
        to: String,
    },

    /// Interactive mode: type a word to explain
    Ask,

    /// Daemon mode: watch clipboard for auto-explain
    Daemon,

    /// Explain the current selection — bind to a global hotkey
    Grab {
        /// Where to read text: auto (primary → clipboard), primary, or clipboard
        #[arg(long, default_value = "auto")]
        source: String,

        /// Show detailed explanation
        #[arg(long)]
        expand: bool,

        /// Suppress terminal output; only show a desktop notification
        #[arg(short = 'q', long)]
        quiet: bool,
    },

    /// Show current configuration
    Config,

    /// First-run configuration wizard
    Init,

    /// Show query history
    History {
        /// Number of recent entries to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Search history for a word
        #[arg(short = 's', long)]
        search: Option<String>,

        /// Show query statistics
        #[arg(short = 't', long)]
        stats: bool,

        /// Clear all history
        #[arg(long)]
        clear: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Interactive TUI browser for history
    Tui,
    /// Launch web dashboard for viewing history
    Web {
        /// Port to listen on
        #[arg(short, long, default_value = "9876")]
        port: u16,
    },
}
