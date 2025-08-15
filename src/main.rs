use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod llm;
mod io;
mod util;
mod render;
mod session;
use crate::render as render_mod;
use llm::ProviderRegistry;
use anyhow::Context as _;
use std::time::Duration;
use std::collections::HashMap;
use std::process::Command as StdCommand;

#[derive(Parser, Debug, Clone)]
#[command(name = "sw", version, about = "CLI AI software assistant", long_about = None)]
struct Cli {
    /// Active profile name
    #[arg(short = 'p', long = "profile", global = true)]
    profile: Option<String>,

    /// Default model override
    #[arg(short = 'm', long = "model", global = true)]
    model: Option<String>,

    /// Output JSON instead of human-readable text
    #[arg(long = "json", global = true)]
    json: bool,

    /// Disable ANSI colors
    #[arg(long = "no-color", global = true)]
    no_color: bool,

    /// Increase verbosity (-v, -vv)
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Timeout (seconds) for network requests
    #[arg(long = "timeout", global = true)]
    timeout_secs: Option<u64>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// First-time interactive setup
    Init(InitArgs),

    /// Ask a one-shot question
    Ask(AskArgs),

    /// Interactive multi-turn chat
    Chat(ChatArgs),

    /// Summarize a file
    Summarize(SummarizeArgs),

    /// Explain a code region
    Explain(ExplainArgs),

    /// Review a unified diff/patch file
    Review(ReviewArgs),

    /// Generate a conventional commit message from a diff
    CommitMsg(CommitMsgArgs),

    /// Extract TODOs/action items from a file
    Todos(TodosArgs),

    /// Planning assistance
    Plan(PlanArgs),

    /// Models related commands
    Models {
        #[command(subcommand)]
        command: ModelsCommands,
    },

    /// Manage conversation sessions
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    /// Search for patterns in code using ripgrep
    Grep(GrepArgs),

    /// Proposal-driven assistant for development actions
    Agent(AgentArgs),

    /// Propose and apply code diffs with approval
    Diff {
        #[command(subcommand)]
        command: DiffCommands,
    },

    /// Generate and run bash scripts with approval
    Script {
        #[command(subcommand)]
        command: ScriptCommands,
    },

    /// Generate code directly to files (bypass diff safety)
    Generate {
        /// Instruction for what code to generate
        #[arg(long)]
        instruction: String,
        /// File to generate
        #[arg(long)]
        file: Option<PathBuf>,
        /// Multiple files to generate
        #[arg(long = "files")]
        files: Vec<PathBuf>,
        /// Provider to use (overrides profile default)
        #[arg(long)]
        provider: Option<String>,
    },

    /// Enhanced file operations with git-awareness and pattern matching
    Files {
        #[command(subcommand)]
        command: FilesCommands,
    },

    /// Checkpoint management for safe experimentation
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
    },

    /// Batch file operations
    Batch {
        #[command(subcommand)]
        command: BatchCommands,
    },
    /// Project templates and scaffolding
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
}

#[derive(Args, Debug, Clone)]
struct InitArgs {
    /// Non-interactive: provider name (e.g., openai)
    #[arg(long)]
    provider: Option<String>,
    /// Non-interactive: API key value or env var ref
    #[arg(long = "api-key")] 
    api_key: Option<String>,
    /// Non-interactive: default model
    #[arg(long)]
    default_model: Option<String>,
    /// Profile name to create or update (default: "default")
    #[arg(long, default_value = "default")]
    profile: String,
    /// Validate credentials now (non-interactive). Interactive mode will prompt.
    #[arg(long)]
    validate: bool,
}

#[derive(Args, Debug, Clone)]
struct AskArgs {
    /// Question to ask
    #[arg(required = true, num_args = 1.., value_name = "PROMPT...")]
    prompt: Vec<String>,
    /// Stream output tokens as they arrive
    #[arg(long)]
    stream: bool,
    /// Associate with a named session
    #[arg(long)]
    session: Option<String>,
    /// Provider to use (e.g., openai, mock)
    #[arg(long)]
    provider: Option<String>,
}

#[derive(Args, Debug, Clone)]
struct ChatArgs {
    /// Start or continue a named session
    #[arg(long)]
    session: Option<String>,
}

#[derive(Args, Debug, Clone)]
struct SummarizeArgs {
    /// Path to file to summarize
    #[arg(long)]
    file: PathBuf,
    /// Max tokens hint to the model
    #[arg(long = "max-tokens")]
    max_tokens: Option<u32>,
    /// Provider to use (e.g., openai, mock)
    #[arg(long, default_value = "openai")]
    provider: String,
}

#[derive(Args, Debug, Clone)]
struct ExplainArgs {
    /// Path to file to explain
    #[arg(long)]
    file: PathBuf,
    /// Optional range: START:END (lines)
    #[arg(long)]
    range: Option<String>,
    /// Provider to use (e.g., openai, mock)
    #[arg(long, default_value = "openai")]
    provider: String,
}

#[derive(Args, Debug, Clone)]
struct ReviewArgs {
    /// Path to unified diff/patch file
    #[arg(long = "diff-file")]
    diff_file: PathBuf,
    /// Provider to use (e.g., openai, mock)
    #[arg(long)]
    provider: Option<String>,
}

#[derive(Args, Debug, Clone)]
struct CommitMsgArgs {
    /// Path to diff/patch file
    #[arg(long = "diff-file")]
    diff_file: PathBuf,
    /// Output as JSON
    #[arg(long)]
    json: bool,
    /// Provider to use (e.g., openai, mock)
    #[arg(long, default_value = "openai")]
    provider: String,
}

#[derive(Args, Debug, Clone)]
struct TodosArgs {
    /// Path to file to scan
    #[arg(long)]
    file: PathBuf,
    /// Provider to use for optional normalization
    #[arg(long)]
    provider: Option<String>,
    /// Normalize with LLM (provider must not be mock)
    #[arg(long)]
    normalize: bool,
}

#[derive(Args, Debug, Clone)]
struct PlanArgs {
    /// Goal text to plan for
    #[arg(long)]
    goal: String,
    /// Optional constraints
    #[arg(long)]
    constraints: Option<String>,
}

#[derive(Args, Debug, Clone)]
struct ModelsListArgs {
    /// Provider to list (e.g., openai, mock)
    #[arg(long)]
    provider: Option<String>,
    /// Force refresh from remote and overwrite cache
    #[arg(long)]
    refresh: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum ModelsCommands {
    /// List available models
    List(ModelsListArgs),
}

#[derive(Subcommand, Debug, Clone)]
enum SessionCommands {
    /// Create a new session and optionally make active
    New { name: String },
    /// List sessions
    List,
    /// Switch active session
    Switch { name: String },
    /// Show active session details
    Show,
    /// Search within a session by substring
    Search { name: String, #[arg(long = "contains")] contains: String },
}

#[derive(Args, Debug, Clone)]
struct GrepArgs {
    /// Pattern to search for
    #[arg(required = true)]
    pattern: String,
    /// Path to search in (defaults to current directory)
    #[arg(long)]
    path: Option<PathBuf>,
    /// File type to search (e.g., rs, py, js, ts, go)
    #[arg(long = "type")]
    file_type: Option<String>,
    /// Use regex pattern matching
    #[arg(long)]
    regex: bool,
    /// Use fixed string matching (not regex)
    #[arg(long)]
    fixed: bool,
    /// Case insensitive search
    #[arg(short = 'i', long)]
    ignore_case: bool,
    /// Number of context lines to show before matches
    #[arg(short = 'B', long)]
    before_context: Option<usize>,
    /// Number of context lines to show after matches
    #[arg(short = 'A', long)]
    after_context: Option<usize>,
    /// Number of context lines to show before and after matches
    #[arg(short = 'C', long)]
    context: Option<usize>,
}

#[derive(Args, Debug, Clone)]
struct AgentArgs {
    /// Question or instruction for the agent
    #[arg(required = true, num_args = 1.., value_name = "INSTRUCTION...")]
    instruction: Vec<String>,
    /// Automatically accept all proposals (non-interactive)
    #[arg(long)]
    yes: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum DiffCommands {
    /// Propose unified diffs from instructions
    Propose {
        /// Instruction for what changes to make
        #[arg(long)]
        instruction: String,
        /// File to modify
        #[arg(long)]
        file: Option<PathBuf>,
        /// Multiple files to modify
        #[arg(long = "files")]
        files: Vec<PathBuf>,
        /// Provider to use (overrides profile default)
        #[arg(long)]
        provider: Option<String>,
    },
    /// Apply a provided diff after approval
    Apply {
        /// Path to diff file to apply
        #[arg(long)]
        file: PathBuf,
        /// Automatically apply without approval (non-interactive)
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum ScriptCommands {
    /// Generate a bash script for a goal
    Gen {
        /// Goal description for script generation
        #[arg(long)]
        goal: Option<String>,
        /// Script file to analyze/explain
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Run a bash script with safety checks and approval
    Run {
        /// Script file to run
        #[arg(long)]
        file: PathBuf,
        /// Show what would run without executing (dry run)
        #[arg(long)]
        dry_run: bool,
        /// Automatically run without approval (non-interactive)
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum FilesCommands {
    /// List files with git-awareness and pattern matching
    List {
        /// Directory to search (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Search recursively
        #[arg(long, short = 'r')]
        recursive: bool,
        /// Disable git-aware filtering (git-aware is enabled by default)
        #[arg(long)]
        no_git: bool,
        /// Include files with specific extensions (e.g., js,ts,py)
        #[arg(long)]
        include_ext: Option<String>,
        /// Exclude files with specific extensions
        #[arg(long)]
        exclude_ext: Option<String>,
        /// Include files matching pattern
        #[arg(long)]
        include_pattern: Option<String>,
        /// Exclude files matching pattern
        #[arg(long)]
        exclude_pattern: Option<String>,
    },
    /// Find git repository root
    GitRoot {
        /// Path to start search from
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Analyze file structure and dependencies
    Analyze {
        /// File or directory to analyze
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Analyze recursively
        #[arg(long, short = 'r')]
        recursive: bool,
        /// Include files with specific extensions (e.g., js,ts,py)
        #[arg(long)]
        include_ext: Option<String>,
        /// Exclude files with specific extensions
        #[arg(long)]
        exclude_ext: Option<String>,
        /// Show detailed function and class information
        #[arg(long)]
        detailed: bool,
        /// Generate dependency graph
        #[arg(long)]
        dependencies: bool,
    },
    /// Compare and synchronize directories
    Compare {
        /// Source directory
        #[arg(long)]
        source: PathBuf,
        /// Target directory
        #[arg(long)]
        target: PathBuf,
        /// Include content comparison
        #[arg(long)]
        content: bool,
        /// Ignore timestamps
        #[arg(long)]
        ignore_time: bool,
        /// Exclude patterns (comma-separated)
        #[arg(long)]
        exclude: Option<String>,
    },
    /// Synchronize directories
    Sync {
        /// Source directory
        #[arg(long)]
        source: PathBuf,
        /// Target directory
        #[arg(long)]
        target: PathBuf,
        /// Dry run (don't actually sync)
        #[arg(long)]
        dry_run: bool,
        /// Include content comparison
        #[arg(long)]
        content: bool,
        /// Exclude patterns (comma-separated)
        #[arg(long)]
        exclude: Option<String>,
    },
    /// Find duplicate files
    Duplicates {
        /// Directory to search
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Search recursively
        #[arg(long, short = 'r')]
        recursive: bool,
    },
    /// Advanced search with content analysis
    Search {
        /// Search pattern
        #[arg(long)]
        pattern: String,
        /// Directory to search
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Case sensitive search
        #[arg(long)]
        case_sensitive: bool,
        /// Regex pattern matching
        #[arg(long)]
        regex: bool,
        /// Fuzzy matching
        #[arg(long)]
        fuzzy: bool,
        /// Semantic search (functions, classes)
        #[arg(long)]
        semantic: bool,
        /// Whole word matching
        #[arg(long)]
        whole_word: bool,
        /// Context lines around matches
        #[arg(long, default_value = "2")]
        context: usize,
        /// File types to include (comma-separated)
        #[arg(long)]
        types: Option<String>,
        /// Maximum matches per file
        #[arg(long)]
        max_matches: Option<usize>,
    },
    /// Search and replace with content analysis
    Replace {
        /// Search pattern
        #[arg(long)]
        pattern: String,
        /// Replacement text
        #[arg(long)]
        replace: String,
        /// Directory to search
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Dry run (don't actually replace)
        #[arg(long)]
        dry_run: bool,
        /// Case sensitive search
        #[arg(long)]
        case_sensitive: bool,
        /// Regex pattern matching
        #[arg(long)]
        regex: bool,
        /// File types to include (comma-separated)
        #[arg(long)]
        types: Option<String>,
    },
    /// Security vulnerability scanning
    Security {
        /// Directory to scan
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Include informational issues
        #[arg(long)]
        include_info: bool,
        /// Check for hardcoded credentials
        #[arg(long, default_value = "true")]
        check_credentials: bool,
        /// Check for injection vulnerabilities
        #[arg(long, default_value = "true")]
        check_injection: bool,
        /// Check for cryptographic issues
        #[arg(long, default_value = "true")]
        check_crypto: bool,
        /// Check for path traversal vulnerabilities
        #[arg(long, default_value = "true")]
        check_paths: bool,
        /// Check for dependency vulnerabilities
        #[arg(long, default_value = "true")]
        check_dependencies: bool,
        /// Check for configuration issues
        #[arg(long, default_value = "true")]
        check_configuration: bool,
        /// File types to include (comma-separated)
        #[arg(long)]
        types: Option<String>,
        /// Show only high severity and above
        #[arg(long)]
        high_only: bool,
        /// Minimum risk score to display
        #[arg(long)]
        min_risk: Option<u32>,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum CheckpointCommands {
    /// Create a new checkpoint
    Create {
        /// Description for the checkpoint
        #[arg(long)]
        description: String,
        /// Files to include in checkpoint
        #[arg(long = "files")]
        files: Vec<PathBuf>,
    },
    /// List available checkpoints
    List,
    /// Restore from a checkpoint
    Restore {
        /// Checkpoint ID to restore
        #[arg(long)]
        id: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum BatchCommands {
    /// Process multiple files with code generation
    Generate {
        /// Instruction for code generation
        #[arg(long)]
        instruction: String,
        /// Directory to search for files
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Search recursively
        #[arg(long, short = 'r')]
        recursive: bool,
        /// Include files with specific extensions
        #[arg(long)]
        include_ext: Option<String>,
        /// Exclude files with specific extensions
        #[arg(long)]
        exclude_ext: Option<String>,
        /// Provider to use
        #[arg(long)]
        provider: Option<String>,
        /// Create checkpoint before processing
        #[arg(long)]
        checkpoint: bool,
    },
    /// Apply code changes to multiple files
    Transform {
        /// Transformation instruction
        #[arg(long)]
        instruction: String,
        /// Directory to search for files
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Search recursively
        #[arg(long, short = 'r')]
        recursive: bool,
        /// Include files with specific extensions
        #[arg(long)]
        include_ext: Option<String>,
        /// Provider to use
        #[arg(long)]
        provider: Option<String>,
        /// Create checkpoint before processing
        #[arg(long)]
        checkpoint: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum TemplateCommands {
    /// List available templates
    List,
    /// Generate project from template
    Generate {
        /// Template name to use
        #[arg(long)]
        template: String,
        /// Output directory for generated project
        #[arg(long, default_value = ".")]
        output: PathBuf,
        /// Project name
        #[arg(long)]
        name: String,
        /// Author name
        #[arg(long, default_value = "Unknown")]
        author: String,
        /// Template variables in key=value format
        #[arg(long)]
        var: Vec<String>,
    },
}

#[derive(Debug, Clone)]
struct GlobalOpts {
    profile: Option<String>,
    model: Option<String>,
    json: bool,
    no_color: bool,
    verbose: u8,
    timeout_secs: Option<u64>,
}

fn json_error(_globals: &GlobalOpts, _code: &str, message: &str, _hint: Option<&str>) -> anyhow::Error {
    // Do not print here; the top-level handler prints exactly once in --json mode
    anyhow::anyhow!(message.to_string())
}

fn derive_error_code(err: &anyhow::Error) -> (&'static str, Option<&'static str>) {
    let msg = err.to_string();
    if msg.contains("file not found") { return ("file_not_found", None); }
    if msg.contains("empty diff file") || msg.contains("empty prompt") || msg.contains("empty goal") { return ("missing_input", None); }
    if msg.contains("invalid --range") || msg.contains("invalid range") { return ("invalid_args", None); }
    if msg.contains("OPENAI_API_KEY") { return ("missing_api_key", Some("set OPENAI_API_KEY in env or .env")); }
    if msg.contains("timed out") { return ("timeout", Some("try increasing --timeout")); }
    if msg.contains("unsupported provider") { return ("provider_unsupported", None); }
    if msg.contains("failed to parse") || msg.to_lowercase().contains("parse error") { return ("parse_error", None); }
    if msg.to_lowercase().contains("network") || msg.contains("dns") || msg.contains("Connection") { return ("network_error", None); }
    ("unknown", None)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let Cli {
        profile,
        model,
        json,
        no_color,
        verbose,
        timeout_secs,
        command,
    } = cli;

    let globals = GlobalOpts {
        profile,
        model,
        json,
        no_color,
        verbose,
        timeout_secs,
    };

    let result = match command {
        Commands::Init(args) => cmd_init(&globals, args).await,
        Commands::Ask(args) => cmd_ask(&globals, args).await,
        Commands::Chat(args) => cmd_chat(&globals, args).await,
        Commands::Summarize(args) => cmd_summarize(&globals, args).await,
        Commands::Explain(args) => cmd_explain(&globals, args).await,
        Commands::Review(args) => cmd_review(&globals, args).await,
        Commands::CommitMsg(args) => cmd_commit_msg(&globals, args).await,
        Commands::Todos(args) => cmd_todos(&globals, args).await,
        Commands::Plan(args) => cmd_plan(&globals, args).await,
        Commands::Models { command } => cmd_models(&globals, command).await,
        Commands::Session { command } => cmd_session(&globals, command).await,
        Commands::Grep(args) => cmd_grep(&globals, args).await,
        Commands::Agent(args) => cmd_agent(&globals, args).await,
        Commands::Diff { command } => cmd_diff(&globals, command).await,
        Commands::Script { command } => cmd_script(&globals, command).await,
        Commands::Generate { instruction, file, files, provider } => cmd_generate(&globals, instruction, file, files, provider).await,
        Commands::Files { command } => cmd_files(&globals, command).await,
        Commands::Checkpoint { command } => cmd_checkpoint(&globals, command).await,
        Commands::Batch { command } => cmd_batch(&globals, command).await,
        Commands::Template { command } => cmd_template(&globals, command).await,
    };

    if let Err(e) = result {
        if globals.json {
            let (code, hint) = classify_error(&e);
            let msg = e.to_string();
            render_mod::print_json_error(&code, &msg, hint.as_deref());
        } else {
            eprintln!("{}", e);
        }
        std::process::exit(1);
    }

    Ok(())
}

fn classify_error(e: &anyhow::Error) -> (String, Option<String>) {
    let msg = e.to_string().to_lowercase();
    if msg.contains("file not found") {
        return ("file_not_found".to_string(), Some("check the file path".to_string()));
    }
    if msg.contains("empty diff file") || msg.contains("empty goal") {
        return ("missing_input".to_string(), None);
    }
    if msg.contains("invalid --range") || msg.contains("invalid range") || msg.contains("invalid start") || msg.contains("invalid end") {
        return ("invalid_args".to_string(), None);
    }
    if msg.contains("missing openai_api_key") {
        return ("missing_api_key".to_string(), Some("set OPENAI_API_KEY in env or .env".to_string()));
    }
    if msg.contains("timed out") || msg.contains("timeout") {
        return ("timeout".to_string(), Some("try increasing --timeout or check network".to_string()));
    }
    if msg.contains("unsupported provider") {
        return ("provider_unsupported".to_string(), None);
    }
    if msg.contains("approval required") {
        return ("approval_required".to_string(), Some("re-run with --yes to approve".to_string()));
    }
    if msg.contains("blocked action") {
        return ("blocked_action".to_string(), None);
    }
    if msg.contains("network") || msg.contains("dns") || msg.contains("connection refused") {
        return ("network_error".to_string(), None);
    }
    if msg.contains("session not found") {
        return ("session_not_found".to_string(), None);
    }
    ("unknown".to_string(), None)
}

fn resolve_api_base_for_provider(provider: &str) -> Option<String> {
    match provider.to_lowercase().as_str() {
        "groq" => Some("https://api.groq.com/openai/v1".to_string()),
        "lmstudio" => std::env::var("LMSTUDIO_API_BASE").ok().or_else(|| Some("http://127.0.0.1:1234/v1".to_string())),
        _ => None,
    }
}

async fn cmd_init(_globals: &GlobalOpts, mut args: InitArgs) -> anyhow::Result<()> {
    use config::{default_config_path, load_config_if_exists, write_config, Profile};
    use std::io::{IsTerminal as _, Write};

    let path = default_config_path()?;
    let mut cfg = load_config_if_exists(&path)?.unwrap_or_default();

    // Interactive prompts when missing inputs and in TTY
    let stdin_is_tty = std::io::stdin().is_terminal();
    let stdout_is_tty = std::io::stdout().is_terminal();
    let interactive = stdin_is_tty && stdout_is_tty;

    // Determine provider
    if args.provider.is_none() && interactive {
        print!("Provider [openai|groq|lmstudio|mock] (default: openai): ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let p = line.trim();
        args.provider = Some(if p.is_empty() { "openai".to_string() } else { p.to_string() });
    }
    let provider = args.provider.clone().unwrap_or_else(|| "openai".to_string());

    // Determine API key (skip for lmstudio/mock); prefer given arg; otherwise, use env if present; interactive prompt if still missing
    let needs_key = !matches!(provider.to_lowercase().as_str(), "lmstudio" | "mock");
    if needs_key && args.api_key.is_none() {
        // Try env var per provider
        let env_key_name = match provider.to_lowercase().as_str() {
            "openai" => "OPENAI_API_KEY",
            "groq" => "GROQ_API_KEY",
            _ => "",
        };
        if !env_key_name.is_empty() {
            if let Ok(val) = std::env::var(env_key_name) {
                if !val.trim().is_empty() {
                    args.api_key = Some(val);
                }
            }
        }
        if args.api_key.is_none() && interactive {
            print!("API key for {} (leave blank to skip): ", provider);
            std::io::stdout().flush().ok();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let v = line.trim();
            if !v.is_empty() { args.api_key = Some(v.to_string()); }
        }
    }

    // Determine model
    if args.default_model.is_none() && interactive {
        let suggestion = match provider.to_lowercase().as_str() {
            "openai" => "gpt-4o-mini",
            "groq" => "llama3-8b-8192",
            "lmstudio" => "gpt-4o-mini",
            _ => "gpt-4o-mini",
        };
        print!("Default model (default: {}): ", suggestion);
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let m = line.trim();
        args.default_model = Some(if m.is_empty() { suggestion.to_string() } else { m.to_string() });
    }

    // Choose profile name behavior if exists
    let mut profile_name = args.profile.clone();
    if interactive {
        if cfg.profiles.contains_key(&profile_name) {
            println!("Profile '{}' already exists.", profile_name);
            print!("Press Enter to overwrite, or type a new profile name: ");
            std::io::stdout().flush().ok();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let s = line.trim();
            if !s.is_empty() { profile_name = s.to_string(); }
        }
    }

    // Optional validation
    let mut do_validate = args.validate;
    if interactive && !do_validate {
        print!("Validate credentials now? [y/N]: ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let a = line.trim().to_lowercase();
        do_validate = a == "y" || a == "yes";
    }
    if do_validate {
        let api_base = resolve_api_base_for_provider(&provider);
        let key_opt = args.api_key.as_deref();
        llm::validate_provider_credentials(&provider, key_opt, api_base.as_deref(), _globals.timeout_secs).await?;
    }

    // Persist config
    let prof = cfg
        .profiles
        .entry(profile_name.clone())
        .or_insert_with(Profile::default);
    if let Some(p) = args.provider { prof.provider = Some(p); }
    if let Some(api_key) = args.api_key { prof.api_key = Some(api_key); }
    if let Some(model) = args.default_model { prof.model = Some(model); }
    if cfg.default_profile.is_none() { cfg.default_profile = Some(profile_name); }

    write_config(&path, &cfg)?;
    println!("config written: {}", path.display());
    Ok(())
}

async fn cmd_ask(globals: &GlobalOpts, args: AskArgs) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let prompt = args.prompt.join(" ");
    if prompt.trim().is_empty() {
        anyhow::bail!("empty prompt; provide text, e.g. sw ask \"What is Rust async?\"");
    }
    // Resolve provider/model from config and CLI
    let eff = config::resolve_effective_settings(
        globals.profile.as_deref(),
        args.provider.as_deref(),
        globals.model.as_deref(),
    )?;

    // Session handling: choose session name
    let session_name = if let Some(s) = &args.session {
        Some(s.clone())
    } else {
        session::get_active_session()?
    };

    // Allow offline testing via mock provider (also appends to session when present)
    if eff.provider.to_lowercase() == "mock" {
        if let Some(name) = session_name {
            // append user and assistant turns
            let user = session::SessionRecord { timestamp_ms: session::now_ms(), role: "user".into(), content: prompt.clone(), model: None, usage: None };
            session::append_record(&name, &user)?;
            let assistant_text = format!("[stub answer] {}", prompt);
            let assistant = session::SessionRecord { timestamp_ms: session::now_ms(), role: "assistant".into(), content: assistant_text.clone(), model: Some(eff.model.clone()), usage: None };
            session::append_record(&name, &assistant)?;
            if globals.json {
                #[derive(serde::Serialize)]
                struct Out<'a> { model: &'a str, usage: Option<serde_json::Value>, answer: String }
                let out = Out { model: &eff.model, usage: None, answer: assistant_text };
                render_mod::print_json(&out);
            } else {
                println!("{}", assistant_text);
            }
        } else {
            if globals.json {
                #[derive(serde::Serialize)]
                struct Out<'a> { model: &'a str, usage: Option<serde_json::Value>, answer: String }
                let out = Out { model: &eff.model, usage: None, answer: format!("[stub answer] {}", prompt) };
                render_mod::print_json(&out);
            } else {
                println!("[stub answer] {}", prompt);
            }
        }
        return Ok(());
    }

    // Build messages with truncation from session
    let messages = if let Some(name) = &session_name {
        let history = session::load_session_history(name)?;
        session::build_messages_with_truncation(&history, &prompt, 4000)
    } else {
        vec![llm::ChatMessage { role: "user".into(), content: prompt.clone() }]
    };
    let model_for_req = eff.model.clone();
    // In JSON mode, force non-streaming to produce a single JSON object output
    let stream = if globals.json { false } else { args.stream };
    let provider_lower = eff.provider.to_lowercase();
    let api_base = resolve_api_base_for_provider(&provider_lower);
    let req = llm::LlmRequest { model: model_for_req, messages, stream, api_base };
    match provider_lower.as_str() {
        "openai" | "groq" | "lmstudio" => {
            let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(globals.timeout_secs.unwrap_or(60)))?;
            let adapter = registry.get("openai").context("unsupported provider: openai")?;
            if stream {
                let mut stream = adapter.send_stream(req).await.map_err(map_provider_error)?;
                use futures_util::StreamExt;
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(t) => print!("{}", t),
                        Err(e) => return Err(map_provider_error(e)),
                    }
                }
                println!();
            } else {
                let res = adapter.send(req).await.map_err(map_provider_error)?;
                if let Some(name) = session_name.clone() {
                    let user = session::SessionRecord { timestamp_ms: session::now_ms(), role: "user".into(), content: prompt.clone(), model: None, usage: None };
                    session::append_record(&name, &user)?;
                    let assistant = session::SessionRecord { timestamp_ms: session::now_ms(), role: "assistant".into(), content: res.content.clone(), model: Some(eff.model.clone()), usage: res.usage.clone() };
                    session::append_record(&name, &assistant)?;
                }
                if globals.json {
                    #[derive(serde::Serialize)]
                    struct Out<'a> { model: &'a str, usage: Option<&'a llm::Usage>, answer: &'a str }
                    let out = Out { model: &eff.model, usage: res.usage.as_ref(), answer: &res.content };
                    render_mod::print_json(&out);
                } else {
                    println!("{}", res.content);
                }
            }
        }
        other => {
            anyhow::bail!("unsupported provider: {}", other);
        }
    }
    Ok(())
}

async fn cmd_chat(globals: &GlobalOpts, args: ChatArgs) -> anyhow::Result<()> {
    use std::io::{self, Write};
    dotenvy::dotenv().ok();

    let session_name = match args.session {
        Some(name) => name,
        None => match session::get_active_session()? { Some(s) => s, None => {
            anyhow::bail!("no session specified and no active session. Use --session NAME or `sw session new NAME`");
        }},
    };
    session::create_session_if_missing(&session_name)?;
    session::set_active_session(&session_name)?;

    println!("chatting in session: {} (Ctrl+C to exit)", &session_name);
    let eff = config::resolve_effective_settings(
        globals.profile.as_deref(),
        None,
        globals.model.as_deref(),
    )?;

    loop {
        print!("> ");
        io::stdout().flush().ok();
        let mut input = String::new();
        let n = std::io::stdin().read_line(&mut input)?;
        if n == 0 { eprintln!("exiting chat; session saved"); break; }
        let prompt = input.trim().to_string();
        if prompt.is_empty() { continue; }
        if matches!(prompt.as_str(), "/exit" | "exit" | "/quit" | "quit") { eprintln!("bye"); break; }

        // Mock path: echo
        if eff.provider.to_lowercase() == "mock" {
            let user = session::SessionRecord { timestamp_ms: session::now_ms(), role: "user".into(), content: prompt.clone(), model: None, usage: None };
            session::append_record(&session_name, &user)?;
            let assistant_text = format!("[stub chat] {}", prompt);
            let assistant = session::SessionRecord { timestamp_ms: session::now_ms(), role: "assistant".into(), content: assistant_text.clone(), model: Some(eff.model.clone()), usage: None };
            session::append_record(&session_name, &assistant)?;
            println!("{}", assistant_text);
            continue;
        }

        let history = session::load_session_history(&session_name)?;
        let messages = session::build_messages_with_truncation(&history, &prompt, 4000);
        let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(globals.timeout_secs.unwrap_or(60)))?;
        let adapter = registry.get("openai").context("unsupported provider: openai")?;
        let api_base = resolve_api_base_for_provider(&eff.provider);
        let req = llm::LlmRequest { model: eff.model.clone(), messages, stream: false, api_base };
        let res = adapter.send(req).await.map_err(map_provider_error)?;
        let user = session::SessionRecord { timestamp_ms: session::now_ms(), role: "user".into(), content: prompt.clone(), model: None, usage: None };
        session::append_record(&session_name, &user)?;
        let assistant = session::SessionRecord { timestamp_ms: session::now_ms(), role: "assistant".into(), content: res.content.clone(), model: Some(eff.model.clone()), usage: res.usage.clone() };
        session::append_record(&session_name, &assistant)?;
        println!("{}", res.content);
    }
    Ok(())
}

async fn cmd_summarize(globals: &GlobalOpts, args: SummarizeArgs) -> anyhow::Result<()> {

    if !args.file.exists() { return Err(json_error(globals, "file_not_found", &format!("file not found: {}", args.file.display()), None)); }
    dotenvy::dotenv().ok();

    let effective = config::resolve_effective_settings(
        globals.profile.as_deref(),
        Some(args.provider.as_str()),
        globals.model.as_deref(),
    ).map_err(|e| { let (code, hint) = derive_error_code(&e); json_error(globals, code, &e.to_string(), hint) })?;

    let text = io::read_file_to_string_async(&args.file).await?;
    let max_tokens_per_chunk = args.max_tokens.unwrap_or(600) as usize;
    let chunks = io::chunk_text_for_token_limit(&text, max_tokens_per_chunk);

    // Deterministic mock path for tests
    if effective.provider.to_lowercase() == "mock" {
        let chunk_summaries: Vec<String> = chunks
            .iter()
            .map(|(_, s)| s.trim().lines().take(1).collect::<Vec<_>>().join(" "))
            .collect();
        let merged = if chunk_summaries.is_empty() {
            String::new()
        } else if chunk_summaries.len() == 1 {
            chunk_summaries[0].clone()
        } else {
            chunk_summaries.join(" | ")
        };
        if globals.json {
            #[derive(serde::Serialize)]
            struct Out<'a> { model: &'a str, chunks: usize, summary: String }
            let out = Out { model: &effective.model, chunks: chunk_summaries.len(), summary: merged };
            render_mod::print_json(&out);
        } else {
            println!("{}", merged);
        }
        return Ok(());
    }

    // Real provider: summarize each chunk concurrently, then synthesize
    let num_chunks = chunks.len();
    let mut tasks = Vec::with_capacity(num_chunks);
    let api_base_for_provider = resolve_api_base_for_provider(&effective.provider);
    for (i, chunk) in chunks.into_iter() {
        let model = effective.model.clone();
        let api_base = api_base_for_provider.clone();
        let prompt = format!(
            "Summarize the following content (part {}/{}). Focus on key points and be concise.\n\n{}",
            i + 1,
            num_chunks,
            chunk
        );
        let messages = vec![llm::ChatMessage { role: "user".into(), content: prompt }];
        tasks.push(tokio::spawn(async move {
            let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(60))?;
            let adapter = registry.get("openai").context("unsupported provider: openai")?;
            let req = llm::LlmRequest { model, messages, stream: false, api_base };
            let res = adapter.send(req).await.map_err(map_provider_error)?;
            anyhow::Ok(res.content)
        }));
    }
    let mut partials = Vec::with_capacity(num_chunks);
    for t in tasks { partials.push(t.await??); }
    let final_summary = if num_chunks <= 1 {
        partials.into_iter().next().unwrap_or_default()
    } else {
        let synthesis = format!("Synthesize a concise overall summary from these parts:\n- {}", partials.join("\n- "));
        let messages = vec![llm::ChatMessage { role: "user".into(), content: synthesis }];
        let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(globals.timeout_secs.unwrap_or(60)))?;
        let adapter = registry.get("openai").context("unsupported provider: openai")?;
        let api_base = resolve_api_base_for_provider(&effective.provider);
        let req = llm::LlmRequest { model: effective.model.clone(), messages, stream: false, api_base };
        adapter.send(req).await.map_err(map_provider_error)?.content
    };

    if globals.json {
        #[derive(serde::Serialize)]
        struct Out<'a> { model: &'a str, chunks: usize, summary: String }
        let out = Out { model: &effective.model, chunks: num_chunks, summary: final_summary };
        render_mod::print_json(&out);
    } else {
        println!("{}", final_summary);
    }
    Ok(())
}

async fn cmd_explain(globals: &GlobalOpts, args: ExplainArgs) -> anyhow::Result<()> {
    if !args.file.exists() { return Err(json_error(globals, "file_not_found", &format!("file not found: {}", args.file.display()), None)); }
    dotenvy::dotenv().ok();

    let effective = config::resolve_effective_settings(
        globals.profile.as_deref(),
        Some(args.provider.as_str()),
        globals.model.as_deref(),
    )?;

    let (snippet, range_label) = if let Some(range) = &args.range {
        let parts: Vec<_> = range.split(':').collect();
        if parts.len() != 2 { return Err(json_error(globals, "invalid_args", "invalid --range, expected START:END", None)); }
        let start: usize = parts[0].parse().map_err(|_| json_error(globals, "invalid_args", "invalid START", None))?;
        let end: usize = parts[1].parse().map_err(|_| json_error(globals, "invalid_args", "invalid END", None))?;
        if start == 0 || end < start { return Err(json_error(globals, "invalid_args", "invalid range values", None)); }
        let text = io::read_file_segment_range_async(&args.file, start, end).await?;
        (text, format!("{}:{}", start, end))
    } else {
        let text = io::read_file_to_string_async(&args.file).await?;
        (text, "full".to_string())
    };
    let language = detect_language_from_path(&args.file);

    if effective.provider.to_lowercase() == "mock" {
        let first = snippet.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
        let explanation = format!("Explanation for {} {} ({}): {}", args.file.display(), range_label, language, first);
        if globals.json {
            #[derive(serde::Serialize)]
            struct Out<'a> { model: &'a str, file: String, range: String, explanation: String }
            let out = Out { model: &effective.model, file: args.file.display().to_string(), range: range_label, explanation };
            render_mod::print_json(&out);
        } else {
            println!("{}", explanation);
        }
        return Ok(());
    }

    let prompt = format!(
        "Explain the following {} code from file {} (range: {}). Include what it does, key functions/structures, and potential pitfalls.\n\n```{}\n{}\n```",
        language,
        args.file.display(),
        range_label,
        language.to_lowercase(),
        snippet
    );
    let registry = ProviderRegistry::new()?;
    let adapter = registry.get("openai").context("unsupported provider: openai")?;
    let messages = vec![llm::ChatMessage { role: "user".into(), content: prompt }];
    let api_base = resolve_api_base_for_provider(&effective.provider);
    let req = llm::LlmRequest { model: effective.model.clone(), messages, stream: false, api_base };
    let res = adapter.send(req).await.map_err(map_provider_error)?;
    let explanation = res.content;

    if globals.json {
        #[derive(serde::Serialize)]
        struct Out<'a> { model: &'a str, file: String, range: String, explanation: String }
        let out = Out { model: &effective.model, file: args.file.display().to_string(), range: range_label, explanation };
        render_mod::print_json(&out);
    } else {
        println!("{}", explanation);
    }
    Ok(())
}

fn detect_language_from_path(path: &PathBuf) -> String {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "rs" => "Rust".to_string(),
        "py" => "Python".to_string(),
        "ts" | "tsx" => "TypeScript".to_string(),
        "js" | "jsx" => "JavaScript".to_string(),
        "md" => "Markdown".to_string(),
        "toml" => "TOML".to_string(),
        "json" => "JSON".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "text".to_string(),
    }
}

async fn cmd_review(globals: &GlobalOpts, args: ReviewArgs) -> anyhow::Result<()> {
    if !args.diff_file.exists() { return Err(json_error(globals, "file_not_found", &format!("diff file not found: {}", args.diff_file.display()), None)); }
    let diff = io::read_diff_file_async(&args.diff_file).await?;
    if diff.trim().is_empty() { return Err(json_error(globals, "missing_input", &format!("empty diff file: {}", args.diff_file.display()), None)); }

    let eff = config::resolve_effective_settings(
        globals.profile.as_deref(),
        args.provider.as_deref(),
        globals.model.as_deref(),
    ).map_err(|e| { let (code, hint) = derive_error_code(&e); json_error(globals, code, &e.to_string(), hint) })?;

    // Fallback to mock behavior if offline
    let provider_lower = eff.provider.to_lowercase();
    let missing_openai_key = std::env::var("OPENAI_API_KEY").is_err();
    let no_explicit_provider = args.provider.is_none();
    if no_explicit_provider || provider_lower == "mock" || (provider_lower == "openai" && missing_openai_key) {
        if globals.json {
            #[derive(serde::Serialize)]
            struct ReviewJson<'a> { feedback: Feedback<'a> }
            #[derive(serde::Serialize)]
            struct Feedback<'a> { correctness: Vec<&'a str>, style: Vec<&'a str>, security: Vec<&'a str>, tests: Vec<&'a str>, suggestions: Vec<&'a str> }
            let out = ReviewJson { feedback: Feedback {
                correctness: vec!["check logic changes"],
                style: vec!["ensure formatting"],
                security: vec!["validate inputs"],
                tests: vec!["add/adjust tests"],
                suggestions: vec!["consider smaller functions"],
            }};
            render_mod::print_json(&out);
        } else {
            let fb = render_mod::Feedback {
                correctness: vec!["check logic changes".into()],
                style: vec!["ensure formatting".into()],
                security: vec!["validate inputs".into()],
                tests: vec!["add/adjust tests".into()],
                suggestions: vec!["consider smaller functions".into()],
            };
            render_mod::render_review_text(&fb);
        }
        return Ok(());
    }

    let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(globals.timeout_secs.unwrap_or(60)))?;
    let prompt = if globals.json {
        format!(
            "You are a senior engineer. Review the unified diff. Return STRICT JSON ONLY with exactly this schema and no extra text or markdown.\\n{{\\n  \"feedback\": {{\\n    \"correctness\": [string],\\n    \"style\": [string],\\n    \"security\": [string],\\n    \"tests\": [string],\\n    \"suggestions\": [string]\\n  }}\\n}}\\nDiff:\n{}",
            diff
        )
    } else {
        let rubric = r#"You are a senior engineer. Review the unified diff with sections:
- correctness: issues or risks
- style: naming, structure, clarity
- security: input validation, injection, secrets
- tests: coverage holes or missing cases
- suggestions: concrete changes
Output compact markdown with these headings only."#;
        format!("{}\n\nDiff:\n{}", rubric, diff)
    };
    let adapter = registry.get("openai").context("unsupported provider: openai").map_err(|e| { let (code, hint) = derive_error_code(&anyhow::anyhow!(e.to_string())); json_error(globals, code, &e.to_string(), hint) })?;
    let messages = vec![llm::ChatMessage { role: "user".into(), content: prompt }];
    let api_base = resolve_api_base_for_provider(&eff.provider);
    let req = llm::LlmRequest { model: eff.model, messages, stream: false, api_base };
    let res = adapter.send(req).await.map_err(map_provider_error).map_err(|e| { let (code, hint) = derive_error_code(&e); json_error(globals, code, &e.to_string(), hint) })?;
    if globals.json {
        // Try strict parse; degrade gracefully to suggestions-only
        #[derive(serde::Deserialize, serde::Serialize)]
        struct ReviewJson { feedback: render_mod::Feedback }
        let parsed = serde_json::from_str::<ReviewJson>(res.content.trim());
        let value = match parsed {
            Ok(v) => v,
            Err(_) => ReviewJson { feedback: render_mod::Feedback {
                correctness: vec![], style: vec![], security: vec![], tests: vec![], suggestions: vec![res.content],
            }},
        };
        render_mod::print_json(&value);
    } else {
        println!("{}", res.content);
    }
    Ok(())
}

fn map_provider_error(e: anyhow::Error) -> anyhow::Error {
    // Basic mapping for user-friendly messages; extend as needed
    let msg = e.to_string();
    if msg.contains("OPENAI_API_KEY") {
        return anyhow::anyhow!("missing OPENAI_API_KEY (set in .env or environment)");
    }
    if msg.contains("timed out") {
        return anyhow::anyhow!("request timed out; try --timeout or check network");
    }
    e
}

async fn cmd_commit_msg(globals: &GlobalOpts, args: CommitMsgArgs) -> anyhow::Result<()> {
    if !args.diff_file.exists() { return Err(json_error(globals, "file_not_found", &format!("diff file not found: {}", args.diff_file.display()), None)); }
    dotenvy::dotenv().ok();
    let effective = config::resolve_effective_settings(
        globals.profile.as_deref(),
        Some(args.provider.as_str()),
        globals.model.as_deref(),
    ).map_err(|e| { let (code, hint) = derive_error_code(&e); json_error(globals, code, &e.to_string(), hint) })?;

    let diff = io::read_diff_file_async(&args.diff_file).await?;
    let is_json = globals.json || args.json;
    if effective.provider.to_lowercase() == "mock" {
        #[derive(serde::Serialize)]
        struct Msg<'a> { r#type: &'a str, scope: Option<&'a str>, subject: &'a str, body: Option<&'a str> }
        let msg = Msg { r#type: "chore", scope: None, subject: "update diff", body: None };
        if is_json {
            render_mod::print_json(&msg);
        } else {
            println!("{}: {}", msg.r#type, msg.subject);
        }
        return Ok(());
    }

    // Build prompt for Conventional Commits JSON output
    let prompt = format!(
        "You are an assistant that writes Conventional Commit messages.\n\
        Given this unified diff, produce ONLY a compact JSON object with the fields:\n\
        {{\n  \"type\": \"feat|fix|chore|docs|refactor|test|perf|build|ci|style|revert\",\n  \"scope\": string|null,\n  \"subject\": string,\n  \"body\": string|null\n}}\n\nDiff:\n{}",
        diff
    );
    let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(globals.timeout_secs.unwrap_or(60)))?;
    let adapter = registry.get("openai").context("unsupported provider: openai").map_err(|e| { let (code, hint) = derive_error_code(&anyhow::anyhow!(e.to_string())); json_error(globals, code, &e.to_string(), hint) })?;
    let messages = vec![llm::ChatMessage { role: "user".into(), content: prompt }];
    let api_base = resolve_api_base_for_provider(&effective.provider);
    let req = llm::LlmRequest { model: effective.model.clone(), messages, stream: false, api_base };
    let res = adapter.send(req).await.map_err(map_provider_error).map_err(|e| { let (code, hint) = derive_error_code(&e); json_error(globals, code, &e.to_string(), hint) })?;

    #[derive(serde::Deserialize, serde::Serialize)]
    struct CommitOut { #[serde(rename = "type")] kind: String, scope: Option<String>, subject: String, body: Option<String> }

    // Try to parse JSON from the model output, forgiving code fences
    let parsed: CommitOut = {
        let s = res.content.trim();
        let start = s.find('{').unwrap_or(0);
        let end = s.rfind('}').map(|i| i + 1).unwrap_or_else(|| s.len());
        let json_slice = &s[start..end];
        serde_json::from_str(json_slice)?
    };

    if is_json {
        // Re-map key back to `type` for output
        #[derive(serde::Serialize)]
        struct PublicOut { #[serde(rename = "type")] r#type: String, scope: Option<String>, subject: String, body: Option<String> }
        let out = PublicOut { r#type: parsed.kind, scope: parsed.scope, subject: parsed.subject, body: parsed.body };
        render_mod::print_json(&out);
    } else {
        if let Some(scope) = parsed.scope.as_ref() {
            println!("{}({}): {}", parsed.kind, scope, parsed.subject);
        } else {
            println!("{}: {}", parsed.kind, parsed.subject);
        }
        if let Some(body) = parsed.body.as_ref() {
            if !body.trim().is_empty() {
                println!("\n{}", body.trim());
            }
        }
    }
    Ok(())
}

async fn cmd_todos(globals: &GlobalOpts, args: TodosArgs) -> anyhow::Result<()> {
    if !args.file.exists() { return Err(json_error(globals, "file_not_found", &format!("file not found: {}", args.file.display()), None)); }
    dotenvy::dotenv().ok();
    let text = io::read_file_to_string_async(&args.file).await?;
    let items: Vec<(usize, String)> = io::scan_todos(&text);

    // Optional normalization via LLM (non-mock only)
    if args.normalize {
        let eff = config::resolve_effective_settings(
            globals.profile.as_deref(),
            args.provider.as_deref(),
            globals.model.as_deref(),
        )?;
        if eff.provider.to_lowercase() != "mock" && !items.is_empty() {
            let prompt = format!(
                "Normalize the following TODO/FIXME/NOTE lines into JSON with fields: line, text, priority(one of high|medium|low|null), owner(optional like @user).\nReturn a JSON array only.\n\n{}",
                items.iter().map(|(ln, s)| format!("{}: {}", ln, s)).collect::<Vec<_>>().join("\n")
            );
            let messages = vec![llm::ChatMessage { role: "user".into(), content: prompt }];
            let api_base = resolve_api_base_for_provider(&eff.provider);
            let req = llm::LlmRequest { model: eff.model.clone(), messages, stream: false, api_base };
            let registry = ProviderRegistry::new_with_timeout(Duration::from_secs(globals.timeout_secs.unwrap_or(60)))?;
            let adapter = registry.get("openai").context("unsupported provider: openai")?;
            if let Ok(res) = adapter.send(req).await.map_err(map_provider_error) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&res.content) {
                    if let Some(arr) = parsed.as_array() {
                        // Replace with normalized texts preserving line numbers when present
                        let mut normalized: Vec<(usize, String, Option<String>, Option<String>)> = Vec::new();
                        for v in arr {
                            let line = v.get("line").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                            let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("").to_string();
                            let priority = v.get("priority").and_then(|x| x.as_str()).map(|s| s.to_string());
                            let owner = v.get("owner").and_then(|x| x.as_str()).map(|s| s.to_string());
                            if line != 0 && !text.is_empty() { normalized.push((line, text, priority, owner)); }
                        }
                        // If normalization produced something useful, render that path now
                        if !normalized.is_empty() {
                            if globals.json {
                                #[derive(serde::Serialize)]
                                struct TodoNorm { line: usize, text: String, priority: Option<String>, owner: Option<String> }
                                let out: Vec<_> = normalized.into_iter().map(|(l, t, p, o)| TodoNorm { line: l, text: t, priority: p, owner: o }).collect();
                                render_mod::print_json(&out);
                            } else {
                                for (ln, s, _p, _o) in normalized { println!("{}:{}", ln, s); }
                            }
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    let hits = items;
    if globals.json {
        #[derive(serde::Serialize)]
        struct Todo<'a> { line: usize, text: &'a str, priority: Option<String>, owner: Option<String> }
        let list: Vec<_> = hits.iter().map(|(ln, s)| {
            let up = s.to_uppercase();
            let priority = if up.contains("[PRIO:HIGH]") || up.contains("FIXME") || up.contains("BUG") || up.contains("URGENT") || up.contains(" P0") { Some("high".to_string()) }
                else if up.contains("[PRIO:MED]") || up.contains(" P1") || up.contains("MEDIUM") || up.contains("HACK") || up.contains("OPTIMIZE") { Some("medium".to_string()) }
                else if up.contains("[PRIO:LOW]") || up.contains("TODO") || up.contains("LOW") || up.contains("- [ ]") { Some("low".to_string()) }
                else { None };
            let owner = s.split_whitespace().find(|w| w.starts_with('@')).map(|w| w.trim_matches(|c: char| c == ',' || c == ';' || c == '.').to_string());
            Todo { line: *ln, text: s.as_str(), priority, owner }
        }).collect();
        render_mod::print_json(&list);
    } else {
        if hits.is_empty() {
            println!("(no TODOs found)");
        } else {
            for (ln, s) in hits { println!("{}:{}", ln, s); }
        }
    }
    Ok(())
}

async fn cmd_plan(_globals: &GlobalOpts, args: PlanArgs) -> anyhow::Result<()> {
    if args.goal.trim().is_empty() {
        anyhow::bail!("empty goal; pass --goal text");
    }
    dotenvy::dotenv().ok();
    // Mock path for tests (no provider flag yet: use model/profile only)
    // If no OPENAI_API_KEY, treat as mock
    let use_mock = std::env::var("OPENAI_API_KEY").is_err();
    if use_mock {
        if _globals.json {
            #[derive(serde::Serialize)]
            struct Out<'a> { steps: Vec<&'a str>, risks: Vec<&'a str>, success_criteria: Vec<&'a str> }
            let out = Out { steps: vec!["analyze input", "design tasks", "execute", "validate"], risks: vec!["scope creep"], success_criteria: vec!["tests pass", "meets requirements"] };
            render_mod::print_json(&out);
        } else {
            println!("1) analyze input\n2) design tasks\n3) execute\n4) validate\n\nRisks: scope creep\nSuccess: tests pass; meets requirements");
        }
        return Ok(());
    }
    // Real provider
    let prompt = format!(
        "You are a senior engineer. Create a step-by-step implementation plan for the goal below, listing steps, major risks, and success criteria.\nReturn ONLY compact JSON with keys steps (array of strings), risks (array of strings), and success_criteria (array of strings).\n\nGoal: {}\n\nConstraints: {}",
        args.goal,
        args.constraints.clone().unwrap_or_default()
    );
    let eff = config::resolve_effective_settings(None, Some("openai"), None)?;
    let registry = ProviderRegistry::new()?;
    let adapter = registry.get("openai").context("unsupported provider: openai")?;
    let messages = vec![llm::ChatMessage { role: "user".into(), content: prompt }];
    let req = llm::LlmRequest { model: eff.model, messages, stream: false, api_base: None };
    let res = adapter.send(req).await.map_err(map_provider_error)?;
    let s = res.content.trim();
    let start = s.find('{').unwrap_or(0);
    let end = s.rfind('}').map(|i| i + 1).unwrap_or_else(|| s.len());
    let json_slice = &s[start..end];
    if _globals.json {
        println!("{}", json_slice);
    } else {
        // Best-effort pretty print
        match serde_json::from_str::<serde_json::Value>(json_slice) {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v)?),
            Err(_) => println!("{}", res.content),
        }
    }
    Ok(())
}

async fn cmd_models(globals: &GlobalOpts, cmd: ModelsCommands) -> anyhow::Result<()> {
    match cmd {
        ModelsCommands::List(args) => models_list(globals, args).await,
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct ModelInfo {
    name: String,
    provider: String,
    source: String, // config|remote|cache
    streaming: bool,
    context_window: Option<u32>,
    #[serde(default)]
    supports_json: bool,
    #[serde(default)]
    supports_tools: bool,
    #[serde(default)]
    modalities: Vec<String>, // e.g., ["text"], ["text","vision"]
}

fn cache_models_path() -> anyhow::Result<std::path::PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| anyhow::anyhow!("unable to resolve OS cache directory"))?;
    Ok(base.join("sw-assistant").join("models.json"))
}

async fn models_list(globals: &GlobalOpts, args: ModelsListArgs) -> anyhow::Result<()> {
    use anyhow::Context as _;
    dotenvy::dotenv().ok();

    // Merge effective provider and model from config + CLI
    let eff = config::resolve_effective_settings(
        globals.profile.as_deref(),
        args.provider.as_deref(),
        globals.model.as_deref(),
    )?;

    // Load config to consult capability overrides
    let cfg_path = config::default_config_path()?;
    let cfg_opt = config::load_config_if_exists(&cfg_path)?;

    // Models from config (if any)
    let mut models: Vec<ModelInfo> = Vec::new();
    if !eff.model.is_empty() {
        let (supports_json, supports_tools, modalities) = infer_caps_for_provider_model(&eff.provider, &eff.model);
        let mut mi = ModelInfo { name: eff.model.clone(), provider: eff.provider.clone(), source: "config".to_string(), streaming: true, context_window: None, supports_json, supports_tools, modalities };
        if let Some(cfg) = cfg_opt.as_ref() {
            if let Some(ovr) = cfg.find_model_override(&eff.provider, &eff.model) {
                apply_override(&mut mi, ovr);
            }
        }
        models.push(mi);
    }

    // Try remote fetch
    let mut fetched: Vec<ModelInfo> = Vec::new();
    let provider_lower = eff.provider.to_lowercase();
    let fetch_result: anyhow::Result<Vec<String>> = if args.refresh {
        match provider_lower.as_str() {
            "mock" => Ok(vec!["mock-small".to_string(), "mock-medium".to_string(), "mock-large".to_string()]),
            "openai" => {
                let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
                let http = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15)))
                    .build()?;
                let url = "https://api.openai.com/v1/models";
                let res = http.get(url).bearer_auth(api_key).send().await?;
                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    anyhow::bail!("openai list models failed {}: {}", status, body);
                }
                #[derive(serde::Deserialize)]
                struct OpenAiModels { data: Vec<OpenAiModel> }
                #[derive(serde::Deserialize)]
                struct OpenAiModel { id: String }
                let om: OpenAiModels = res.json().await?;
                Ok(om.data.into_iter().map(|m| m.id).collect())
            }
            "anthropic" => {
                let api_key = std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY not set")?;
                let http = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15)))
                    .build()?;
                let url = "https://api.anthropic.com/v1/models";
                let res = http.get(url)
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .send().await?;
                if !res.status().is_success() {
                    let status = res.status(); let body = res.text().await.unwrap_or_default();
                    anyhow::bail!("anthropic list models failed {}: {}", status, body);
                }
                #[derive(serde::Deserialize)]
                struct AModels { data: Vec<AModel> }
                #[derive(serde::Deserialize)]
                struct AModel { id: String }
                let am: AModels = res.json().await?;
                Ok(am.data.into_iter().map(|m| m.id).collect())
            }
            "groq" => {
                let api_key = std::env::var("GROQ_API_KEY").context("GROQ_API_KEY not set")?;
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15))).build()?;
                let res = http.get("https://api.groq.com/openai/v1/models").bearer_auth(api_key).send().await?;
                if !res.status().is_success() { let s = res.status(); let b = res.text().await.unwrap_or_default(); anyhow::bail!("groq list models failed {}: {}", s, b); }
                #[derive(serde::Deserialize)] struct O { data: Vec<I> } #[derive(serde::Deserialize)] struct I { id: String }
                let o: O = res.json().await?; Ok(o.data.into_iter().map(|i| i.id).collect())
            }
            "gemini" | "google" => {
                let api_key = std::env::var("GOOGLE_API_KEY").context("GOOGLE_API_KEY not set")?;
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15))).build()?;
                let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);
                let res = http.get(url).send().await?;
                if !res.status().is_success() { let s = res.status(); let b = res.text().await.unwrap_or_default(); anyhow::bail!("gemini list models failed {}: {}", s, b); }
                #[derive(serde::Deserialize)] struct G { models: Vec<GModel> } #[derive(serde::Deserialize)] struct GModel { name: String }
                let g: G = res.json().await?; Ok(g.models.into_iter().map(|m| m.name).collect())
            }
            "ollama" => {
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(5))).build()?;
                let res = http.get("http://127.0.0.1:11434/api/tags").send().await?;
                if !res.status().is_success() {
                    let _ = res.text().await;
                    Ok(Vec::new())
                } else {
                    #[derive(serde::Deserialize)] struct Tags { models: Vec<TagModel> } #[derive(serde::Deserialize)] struct TagModel { name: String }
                    let t: Tags = res.json().await.unwrap_or(Tags { models: vec![] });
                    Ok(t.models.into_iter().map(|m| m.name).collect())
                }
            }
            other => anyhow::bail!("unsupported provider: {}", other),
        }
    } else {
        match provider_lower.as_str() {
            "mock" => Ok(vec!["mock-small".to_string(), "mock-medium".to_string(), "mock-large".to_string()]),
            "openai" => {
                let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
                let http = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15)))
                    .build()?;
                let url = "https://api.openai.com/v1/models";
                let res = http.get(url).bearer_auth(api_key).send().await?;
                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    anyhow::bail!("openai list models failed {}: {}", status, body);
                }
                #[derive(serde::Deserialize)]
                struct OpenAiModels { data: Vec<OpenAiModel> }
                #[derive(serde::Deserialize)]
                struct OpenAiModel { id: String }
                let om: OpenAiModels = res.json().await?;
                Ok(om.data.into_iter().map(|m| m.id).collect())
            }
            "anthropic" => {
                let api_key = std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY not set")?;
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15))).build()?;
                let res = http.get("https://api.anthropic.com/v1/models").header("x-api-key", api_key).header("anthropic-version", "2023-06-01").send().await?;
                if !res.status().is_success() { let s = res.status(); let b = res.text().await.unwrap_or_default(); anyhow::bail!("anthropic list models failed {}: {}", s, b); }
                #[derive(serde::Deserialize)] struct A { data: Vec<I> } #[derive(serde::Deserialize)] struct I { id: String }
                let a: A = res.json().await?; Ok(a.data.into_iter().map(|i| i.id).collect())
            }
            
            "groq" => {
                let api_key = std::env::var("GROQ_API_KEY").context("GROQ_API_KEY not set")?;
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15))).build()?;
                let res = http.get("https://api.groq.com/openai/v1/models").bearer_auth(api_key).send().await?;
                if !res.status().is_success() { let s = res.status(); let b = res.text().await.unwrap_or_default(); anyhow::bail!("groq list models failed {}: {}", s, b); }
                #[derive(serde::Deserialize)] struct O { data: Vec<I> } #[derive(serde::Deserialize)] struct I { id: String }
                let o: O = res.json().await?; Ok(o.data.into_iter().map(|i| i.id).collect())
            }
            "gemini" | "google" => {
                let api_key = std::env::var("GOOGLE_API_KEY").context("GOOGLE_API_KEY not set")?;
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(15))).build()?;
                let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);
                let res = http.get(url).send().await?;
                if !res.status().is_success() { let s = res.status(); let b = res.text().await.unwrap_or_default(); anyhow::bail!("gemini list models failed {}: {}", s, b); }
                #[derive(serde::Deserialize)] struct G { models: Vec<M> } #[derive(serde::Deserialize)] struct M { name: String }
                let g: G = res.json().await?; Ok(g.models.into_iter().map(|m| m.name).collect())
            }
            "ollama" => {
                let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(globals.timeout_secs.unwrap_or(5))).build()?;
                let res = http.get("http://127.0.0.1:11434/api/tags").send().await?;
                if !res.status().is_success() {
                    let _ = res.text().await;
                    Ok(Vec::new())
                } else {
                    #[derive(serde::Deserialize)] struct Tags { models: Vec<Tag> } #[derive(serde::Deserialize)] struct Tag { name: String }
                    let t: Tags = res.json().await.unwrap_or(Tags { models: vec![] });
                    Ok(t.models.into_iter().map(|m| m.name).collect())
                }
            }
            other => anyhow::bail!("unsupported provider: {}", other),
        }
    };

    // Cache path
    let cache_path = cache_models_path()?;
    if let Some(parent) = cache_path.parent() { let _ = std::fs::create_dir_all(parent); }

    let ttl_ms: i64 = 24 * 60 * 60 * 1000;
    match fetch_result {
        Ok(names) => {
            // Optional: attempt to enrich capabilities via provider-specific metadata endpoints
            let caps_map: HashMap<String, ModelInfo> = match provider_lower.as_str() {
                "openai" => fetch_openai_model_capabilities(globals.timeout_secs).await.unwrap_or_default(),
                "anthropic" => fetch_anthropic_model_capabilities(globals.timeout_secs).await.unwrap_or_default(),
                "groq" => fetch_groq_model_capabilities(globals.timeout_secs).await.unwrap_or_default(),
                "gemini" | "google" => fetch_gemini_model_capabilities(globals.timeout_secs).await.unwrap_or_default(),
                "ollama" => fetch_ollama_model_capabilities(globals.timeout_secs).await.unwrap_or_default(),
                _ => HashMap::new(),
            };
            for n in names {
                let cw = if n.contains("gpt-4o") { Some(128000) } else { None };
                let (supports_json, supports_tools, modalities) = infer_caps_for_provider_model(&eff.provider, &n);
                let mut mi = ModelInfo { name: n.clone(), provider: eff.provider.clone(), source: "remote".to_string(), streaming: true, context_window: cw, supports_json, supports_tools, modalities };
                if let Some(from_api) = caps_map.get(&n) {
                    // Apply placeholders derived from provider metadata
                    // Only overwrite when metadata provided a concrete value
                    mi.streaming = from_api.streaming;
                    if from_api.context_window.is_some() { mi.context_window = from_api.context_window; }
                    mi.supports_json = from_api.supports_json;
                    mi.supports_tools = from_api.supports_tools;
                    if !from_api.modalities.is_empty() { mi.modalities = from_api.modalities.clone(); }
                }
                if let Some(cfg) = cfg_opt.as_ref() {
                    if let Some(ovr) = cfg.find_model_override(&mi.provider, &mi.name) { apply_override(&mut mi, ovr); }
                }
                fetched.push(mi);
            }
            // Write cache
            let cache_blob = serde_json::json!({
                "timestamp_ms": session::now_ms(),
                "provider": eff.provider,
                "models": fetched
            });
            let _ = std::fs::write(&cache_path, serde_json::to_string_pretty(&cache_blob)?);
        }
        Err(_e) => {
            // Offline fallback: try cache
            if cache_path.exists() {
                let text = std::fs::read_to_string(&cache_path).unwrap_or_default();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    let fresh_enough = val.get("timestamp_ms").and_then(|x| x.as_i64()).map(|ts| session::now_ms() - ts <= ttl_ms).unwrap_or(false);
                    if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
                        if fresh_enough || provider_lower == "mock" {
                            for v in arr {
                                if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
                                    let streaming = v.get("streaming").and_then(|x| x.as_bool()).unwrap_or(true);
                                    let cw = v.get("context_window").and_then(|x| x.as_u64()).map(|x| x as u32);
                                    let supports_json = v.get("supports_json").and_then(|x| x.as_bool()).unwrap_or(false);
                                    let supports_tools = v.get("supports_tools").and_then(|x| x.as_bool()).unwrap_or(false);
                                    let modalities: Vec<String> = v.get("modalities").and_then(|x| x.as_array()).map(|arr| arr.iter().filter_map(|e| e.as_str().map(|s| s.to_string())).collect()).unwrap_or_else(|| vec!["text".to_string()]);
                                    let mut mi = ModelInfo { name: name.to_string(), provider: eff.provider.clone(), source: "cache".to_string(), streaming, context_window: cw, supports_json, supports_tools, modalities };
                                    if let Some(cfg) = cfg_opt.as_ref() {
                                        if let Some(ovr) = cfg.find_model_override(&mi.provider, &mi.name) { apply_override(&mut mi, ovr); }
                                    }
                                    fetched.push(mi);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Merge de-duplicated
    let mut seen = std::collections::BTreeSet::new();
    let mut merged: Vec<ModelInfo> = Vec::new();
    for m in models.into_iter().chain(fetched.into_iter()) {
        if seen.insert(m.name.clone()) { merged.push(m); }
    }

    if globals.json {
        render_mod::print_json(&merged);
    } else {
        if merged.is_empty() {
            println!("(no models found)");
        } else {
            for m in merged {
                let caps = format!(
                    "streaming={} json={} tools={} mods={}",
                    m.streaming, m.supports_json, m.supports_tools, m.modalities.join("+")
                );
                println!(
                    "{}\t{}\t{}\t{}\tcw={}",
                    m.name,
                    m.provider,
                    m.source,
                    caps,
                    m.context_window.map(|v| v.to_string()).unwrap_or_else(|| "unknown".into())
                );
            }
        }
    }
    Ok(())
}

fn infer_caps_for_provider_model(provider: &str, model: &str) -> (bool, bool, Vec<String>) {
    let provider = provider.to_lowercase();
    if provider == "mock" {
        return (true, false, vec!["text".to_string()]);
    }
    if provider == "openai" {
        let name = model.to_lowercase();
        let is_vision = name.contains("gpt-4o") || name.contains("gpt-4.1");
        let supports_tools = name.contains("gpt-4o") || name.contains("gpt-4.1") || name.contains("o-mini");
        let supports_json = name.contains("gpt-4o") || name.contains("gpt-4.1") || name.contains("mini");
        let modalities = if is_vision { vec!["text".to_string(), "vision".to_string()] } else { vec!["text".to_string()] };
        return (supports_json, supports_tools, modalities);
    }
    (false, false, vec!["text".to_string()])
}

fn apply_override(mi: &mut ModelInfo, ovr: &config::ModelCapsOverride) {
    if let Some(v) = ovr.streaming { mi.streaming = v; }
    if let Some(v) = ovr.context_window { mi.context_window = Some(v); }
    if let Some(v) = ovr.supports_json { mi.supports_json = v; }
    if let Some(v) = ovr.supports_tools { mi.supports_tools = v; }
    if let Some(v) = ovr.modalities.as_ref() { mi.modalities = v.clone(); }
}

// Provider-specific capabilities enrichment
async fn fetch_openai_model_capabilities(timeout_secs: Option<u64>) -> anyhow::Result<HashMap<String, ModelInfo>> {
    use serde_json::Value as Json;
    let api_key = match std::env::var("OPENAI_API_KEY") { Ok(v) => v, Err(_) => return Ok(HashMap::new()) };
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs.unwrap_or(15)))
        .build()?;
    // List models first
    #[derive(serde::Deserialize)]
    struct OpenAiModels { data: Vec<OpenAiModel> }
    #[derive(serde::Deserialize)]
    struct OpenAiModel { id: String }
    let list_res = http.get("https://api.openai.com/v1/models").bearer_auth(&api_key).send().await?;
    if !list_res.status().is_success() { return Ok(HashMap::new()); }
    let om: OpenAiModels = list_res.json().await.unwrap_or(OpenAiModels { data: vec![] });
    let mut out: HashMap<String, ModelInfo> = HashMap::new();
    // Limit per-model queries to reasonable number to avoid long runs
    for m in om.data.into_iter().take(50) {
        let url = format!("https://api.openai.com/v1/models/{}", m.id);
        if let Ok(resp) = http.get(&url).bearer_auth(&api_key).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Json>().await {
                    let mut mi = ModelInfo {
                        name: m.id.clone(),
                        provider: "openai".to_string(),
                        source: "remote".to_string(),
                        streaming: true,
                        context_window: None,
                        supports_json: false,
                        supports_tools: false,
                        modalities: vec![],
                    };
                    // Try to read nested capabilities or top-level hints
                    // Accept both { capabilities: { ... } } and top-level fields
                    let caps = json.get("capabilities").cloned().unwrap_or(Json::Null);
                    let get_bool = |obj: &Json, key: &str| obj.get(key).and_then(|v| v.as_bool());
                    let get_num = |obj: &Json, key: &str| obj.get(key).and_then(|v| v.as_u64()).map(|v| v as u32);
                    let get_modalities = |obj: &Json, key: &str| obj.get(key).and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|e| e.as_str().map(|s| s.to_string())).collect::<Vec<_>>() ).unwrap_or_else(|| vec![]);

                    let src_objs: [&Json; 2] = [&json, &caps];
                    for o in &src_objs {
                        if let Some(v) = get_bool(o, "streaming") { mi.streaming = v; }
                        if let Some(v) = get_num(o, "context_window") { mi.context_window = Some(v); }
                        if let Some(v) = get_bool(o, "supports_json") { mi.supports_json = v; }
                        if let Some(v) = get_bool(o, "supports_tools") { mi.supports_tools = v; }
                        let mods = get_modalities(o, "modalities");
                        if !mods.is_empty() { mi.modalities = mods; }
                    }
                    out.insert(m.id, mi);
                }
            }
        }
    }
    Ok(out)
}

async fn fetch_anthropic_model_capabilities(timeout_secs: Option<u64>) -> anyhow::Result<HashMap<String, ModelInfo>> {
    use serde_json::Value as Json;
    let api_key = match std::env::var("ANTHROPIC_API_KEY") { Ok(v) => v, Err(_) => return Ok(HashMap::new()) };
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(timeout_secs.unwrap_or(15))).build()?;
    #[derive(serde::Deserialize)] struct A { data: Vec<M> } #[derive(serde::Deserialize)] struct M { id: String }
    let res = http.get("https://api.anthropic.com/v1/models").header("x-api-key", &api_key).header("anthropic-version", "2023-06-01").send().await?;
    if !res.status().is_success() { return Ok(HashMap::new()); }
    let a: A = res.json().await.unwrap_or(A { data: vec![] });
    let mut out = HashMap::new();
    for m in a.data.into_iter().take(50) {
        let url = format!("https://api.anthropic.com/v1/models/{}", m.id);
        if let Ok(resp) = http.get(&url).header("x-api-key", &api_key).header("anthropic-version", "2023-06-01").send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Json>().await {
                    let mut mi = ModelInfo { name: m.id.clone(), provider: "anthropic".to_string(), source: "remote".to_string(), streaming: true, context_window: None, supports_json: false, supports_tools: false, modalities: vec![] };
                    // Anthropic returns input_token_limit/output_token_limit
                    if let Some(v) = json.get("input_token_limit").and_then(|x| x.as_u64()) { mi.context_window = Some(v as u32); }
                    // Tool use generally supported on Claude 3 family
                    let lname = mi.name.to_lowercase();
                    if lname.contains("claude-3") { mi.supports_tools = true; }
                    out.insert(mi.name.clone(), mi);
                }
            }
        }
    }
    Ok(out)
}

async fn fetch_groq_model_capabilities(timeout_secs: Option<u64>) -> anyhow::Result<HashMap<String, ModelInfo>> {
    let api_key = match std::env::var("GROQ_API_KEY") { Ok(v) => v, Err(_) => return Ok(HashMap::new()) };
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(timeout_secs.unwrap_or(15))).build()?;
    #[derive(serde::Deserialize)] struct O { data: Vec<I> } #[derive(serde::Deserialize)] struct I { id: String }
    let res = http.get("https://api.groq.com/openai/v1/models").bearer_auth(&api_key).send().await?;
    if !res.status().is_success() { return Ok(HashMap::new()); }
    let o: O = res.json().await.unwrap_or(O { data: vec![] });
    let mut out = HashMap::new();
    for i in o.data.into_iter().take(50) {
        // Groq doesn't expose capabilities; placeholder streaming true, text-only
        out.insert(i.id.clone(), ModelInfo { name: i.id, provider: "groq".to_string(), source: "remote".to_string(), streaming: true, context_window: None, supports_json: false, supports_tools: false, modalities: vec!["text".to_string()] });
    }
    Ok(out)
}

async fn fetch_gemini_model_capabilities(timeout_secs: Option<u64>) -> anyhow::Result<HashMap<String, ModelInfo>> {
    use serde_json::Value as Json;
    let api_key = match std::env::var("GOOGLE_API_KEY") { Ok(v) => v, Err(_) => return Ok(HashMap::new()) };
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(timeout_secs.unwrap_or(15))).build()?;
    #[derive(serde::Deserialize)] struct G { models: Vec<M> } #[derive(serde::Deserialize)] struct M { name: String }
    let list = http.get(format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key)).send().await?;
    if !list.status().is_success() { return Ok(HashMap::new()); }
    let g: G = list.json().await.unwrap_or(G { models: vec![] });
    let mut out = HashMap::new();
    for m in g.models.into_iter().take(50) {
        let url = format!("https://generativelanguage.googleapis.com/v1beta/{}?key={}", m.name, api_key);
        if let Ok(resp) = http.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Json>().await {
                    let mut mi = ModelInfo { name: m.name.clone(), provider: "gemini".to_string(), source: "remote".to_string(), streaming: true, context_window: None, supports_json: false, supports_tools: false, modalities: vec![] };
                    // try inputTokenLimit / outputTokenLimit
                    if let Some(v) = json.get("inputTokenLimit").and_then(|x| x.as_u64()) { mi.context_window = Some(v as u32); }
                    // supported modalities placeholders if field exists
                    if let Some(arr) = json.get("supportedModalities").and_then(|x| x.as_array()) { mi.modalities = arr.iter().filter_map(|e| e.as_str().map(|s| s.to_lowercase())).collect(); }
                    if mi.modalities.is_empty() { mi.modalities = vec!["text".to_string()]; }
                    out.insert(mi.name.clone(), mi);
                }
            }
        }
    }
    Ok(out)
}

async fn fetch_ollama_model_capabilities(timeout_secs: Option<u64>) -> anyhow::Result<HashMap<String, ModelInfo>> {
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(timeout_secs.unwrap_or(5))).build()?;
    #[derive(serde::Deserialize)] struct Tags { models: Vec<Tag> } #[derive(serde::Deserialize)] struct Tag { name: String }
    let res = http.get("http://127.0.0.1:11434/api/tags").send().await?;
    if !res.status().is_success() { return Ok(HashMap::new()); }
    let t: Tags = res.json().await.unwrap_or(Tags { models: vec![] });
    let mut out = HashMap::new();
    for m in t.models.into_iter() {
        out.insert(m.name.clone(), ModelInfo { name: m.name, provider: "ollama".to_string(), source: "remote".to_string(), streaming: true, context_window: None, supports_json: false, supports_tools: false, modalities: vec!["text".to_string()] });
    }
    Ok(out)
}

async fn cmd_session(_globals: &GlobalOpts, cmd: SessionCommands) -> anyhow::Result<()> {
    use session::*;
    match cmd {
        SessionCommands::New { name } => {
            create_session_if_missing(&name)?;
            set_active_session(&name)?;
            println!("created and activated session: {}", name);
        }
        SessionCommands::List => {
            let metas = list_sessions_metadata()?;
            if _globals.json {
                #[derive(serde::Serialize)]
                struct J<'a> { name: &'a str, lines: usize, size: u64, last_used_ms: Option<i64> }
                let v: Vec<_> = metas.iter().map(|m| J { name: &m.name, lines: m.num_lines, size: m.file_size, last_used_ms: m.last_used_ms }).collect();
                render_mod::print_json(&v);
            } else {
                for m in metas {
                    println!("{}\tlines={}\tsize={}\tlast={}", m.name, m.num_lines, m.file_size, m.last_used_ms.unwrap_or(0));
                }
            }
        }
        SessionCommands::Switch { name } => {
            let path = session_file_path(&name)?;
            if !path.exists() { return Err(json_error(_globals, "invalid_args", &format!("session not found: {}", name), None)); }
            set_active_session(&name)?;
            println!("active session: {}", name);
        }
        SessionCommands::Show => {
            let active = get_active_session()?;
            if _globals.json {
                #[derive(serde::Serialize)]
                struct J<'a> { active: Option<&'a str>, lines: usize, size: u64, last_used_ms: Option<i64> }
                let mut lines = 0usize; let mut size = 0u64; let mut last = None;
                if let Some(name) = active.as_deref() {
                    if let Some(m) = list_sessions_metadata()?.into_iter().find(|m| m.name == name) {
                        lines = m.num_lines; size = m.file_size; last = m.last_used_ms;
                    }
                    let j = J { active: Some(name), lines, size, last_used_ms: last };
                    render_mod::print_json(&j);
                } else {
                    let j = J { active: None, lines: 0, size: 0, last_used_ms: None };
                    render_mod::print_json(&j);
                }
            } else {
                match active {
                    Some(name) => {
                        let metas = list_sessions_metadata()?;
                        if let Some(m) = metas.into_iter().find(|m| m.name == name) {
                            println!("active: {} (lines={}, size={})", m.name, m.num_lines, m.file_size);
                        } else {
                            println!("active: {} (no file yet)", name);
                        }
                    }
                    None => println!("no active session"),
                }
            }
        }
        SessionCommands::Search { name, contains } => {
            let recs = session::search_session(&name, &contains)?;
            if _globals.json {
                render_mod::print_json(&recs);
            } else {
                for r in recs { println!("{}\t{}: {}", r.timestamp_ms, r.role, r.content); }
            }
        }
    }
    Ok(())
}

async fn cmd_script(globals: &GlobalOpts, cmd: ScriptCommands) -> anyhow::Result<()> {
    match cmd {
        ScriptCommands::Gen { goal, file } => script_gen(globals, goal, file).await,
        ScriptCommands::Run { file, dry_run, yes } => script_run(globals, file, dry_run, yes).await,
    }
}

async fn script_gen(
    globals: &GlobalOpts,
    goal: Option<String>,
    file: Option<PathBuf>,
) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let (script, explanation) = if let Some(path) = file {
        // Minimal explanation for existing script
        if !path.exists() {
            return Err(json_error(globals, "file_not_found", &format!("file not found: {}", path.display()), None));
        }
        let content = io::read_file_to_string_async(&path).await?;
        (content, format!("Existing script: {}", path.display()))
    } else {
        let goal_text = goal.unwrap_or_default();
        if goal_text.trim().is_empty() {
            anyhow::bail!("empty goal; pass --goal text or --file PATH");
        }
        let scaffold = format!(
            "#!/usr/bin/env bash\nset -euo pipefail\n# goal: {}\n# generated by sw\n",
            goal_text.trim()
        );
        (scaffold, format!("Script scaffold for goal: {}", goal_text.trim()))
    };

    if globals.json {
        #[derive(serde::Serialize)]
        struct Out<'a> { script: &'a str, explanation: &'a str }
        let out = Out { script: &script, explanation: &explanation };
        render_mod::print_json(&out);
    } else {
        println!("{}\n\n{}", explanation, script);
    }
    Ok(())
}

async fn script_run(
    globals: &GlobalOpts,
    file: PathBuf,
    dry_run: bool,
    yes: bool,
) -> anyhow::Result<()> {
    use std::io::{IsTerminal as _, Write as _};
    if !file.exists() {
        return Err(json_error(globals, "file_not_found", &format!("file not found: {}", file.display()), None));
    }
    let script = io::read_file_to_string_async(&file).await?;
    validate_script_safety(&script)?;

    if dry_run {
        if globals.json {
            #[derive(serde::Serialize)]
            struct Out<'a> { script: &'a str, would_run: bool }
            let out = Out { script: &script, would_run: true };
            render_mod::print_json(&out);
        } else {
            println!("(dry-run) would run script:\n\n{}", script);
        }
        return Ok(());
    }

    // Approval gating
    let stdin_is_tty = std::io::stdin().is_terminal();
    let stdout_is_tty = std::io::stdout().is_terminal();
    let interactive = stdin_is_tty && stdout_is_tty;
    if !yes {
        if interactive {
            print!("About to run script '{}'. Proceed? [y/N]: ", file.display());
            std::io::stdout().flush().ok();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let a = line.trim().to_lowercase();
            if !(a == "y" || a == "yes") {
                anyhow::bail!("approval required: user rejected");
            }
        } else {
            return Err(json_error(globals, "approval_required", "approval required: re-run with --yes", Some("--yes")));
        }
    }

    // Execute script with timeout
    let timeout = globals.timeout_secs.unwrap_or(60);
    let (exit_code, stdout_s, stderr_s) = execute_script_captured_with_timeout(&file, Duration::from_secs(timeout)).await?;
    if globals.json {
        #[derive(serde::Serialize)]
        struct Out<'a> { script: &'a str, exit_code: i32, stdout: &'a str, stderr: &'a str }
        let out = Out { script: &script, exit_code, stdout: &stdout_s, stderr: &stderr_s };
        render_mod::print_json(&out);
    } else {
        println!("exit={}\n{}", exit_code, stdout_s);
        if !stderr_s.trim().is_empty() { eprintln!("{}", stderr_s); }
    }
    Ok(())
}

fn validate_script_safety(text: &str) -> anyhow::Result<()> {
    let lower = text.to_lowercase();
    let blocked = [
        "rm -rf /",
        "mkfs",
        "shutdown",
        "reboot",
        ":(){ :|:& };:",
        "dd if=/dev/zero",
        ">| /dev/sd",
    ];
    for pat in &blocked {
        if lower.contains(pat) {
            anyhow::bail!("blocked action: script contains '{}'", pat);
        }
    }
    if lower.contains("| sh") && (lower.contains("curl ") || lower.contains("wget ")) {
        anyhow::bail!("blocked action: piping remote into shell");
    }
    if lower.contains("sudo ") {
        anyhow::bail!("blocked action: sudo requires explicit approval");
    }
    Ok(())
}

async fn execute_script_captured_with_timeout(path: &PathBuf, timeout: Duration) -> anyhow::Result<(i32, String, String)> {
    // Use blocking std::process in a spawn_blocking to avoid requiring tokio::process feature
    let path_clone = path.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let output = std::process::Command::new("bash")
            .arg(path_clone)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();
        output
    });
    let res = tokio::time::timeout(timeout, handle).await;
    match res {
        Ok(Ok(Ok(output))) => {
            let code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok((code, stdout, stderr))
        }
        Ok(Ok(Err(e))) => Err(anyhow::anyhow!(e)),
        Ok(Err(join_err)) => Err(anyhow::anyhow!(format!("script execution join error: {}", join_err))),
        Err(_) => Err(anyhow::anyhow!("request timed out; try --timeout or check network")),
    }
}

async fn cmd_grep(globals: &GlobalOpts, args: GrepArgs) -> anyhow::Result<()> {
    // Detect workspace root (defaults to current directory)
    let search_path = args.path.unwrap_or_else(|| detect_workspace_root());
    
    // Build ripgrep command
    let mut cmd = StdCommand::new("rg");
    cmd.arg(&args.pattern);
    
    // Set search path
    cmd.arg(&search_path);
    
    // Add flags based on arguments
    if args.ignore_case {
        cmd.arg("--ignore-case");
    }
    
    if args.fixed {
        cmd.arg("--fixed-strings");
    } else if args.regex {
        // regex is the default for ripgrep, but be explicit
        cmd.arg("--regexp");
    }
    
    if let Some(file_type) = &args.file_type {
        cmd.arg("--type").arg(file_type);
    }
    
    // Context flags
    if let Some(context) = args.context {
        cmd.arg("--context").arg(context.to_string());
    } else {
        if let Some(before) = args.before_context {
            cmd.arg("--before-context").arg(before.to_string());
        }
        if let Some(after) = args.after_context {
            cmd.arg("--after-context").arg(after.to_string());
        }
    }
    
    // JSON output mode
    if globals.json {
        cmd.arg("--json");
        let output = cmd.output()
            .with_context(|| "failed to execute ripgrep (rg)")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ripgrep failed: {}", stderr);
        }
        
        // Parse ripgrep JSON output and convert to our format
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();
        
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            if let Ok(ev) = serde_json::from_str::<serde_json::Value>(line) {
                if ev.get("type").and_then(|t| t.as_str()) == Some("match") {
                    let file_text = ev.get("data").and_then(|d| d.get("path")).and_then(|p| p.get("text")).and_then(|t| t.as_str()).map(|s| s.to_string());
                    let line_number = ev.get("data").and_then(|d| d.get("line_number")).and_then(|n| n.as_u64());
                    let line_text = ev.get("data").and_then(|d| d.get("lines")).and_then(|l| l.get("text")).and_then(|t| t.as_str()).map(|s| s.to_string());
                    if let (Some(p), Some(ln), Some(tx)) = (file_text, line_number, line_text) {
                        #[derive(serde::Serialize)]
                        struct GrepMatch { file: String, line: u64, text: String }
                        matches.push(GrepMatch { file: p, line: ln, text: tx });
                    }
                }
            }
        }
        
        render_mod::print_json(&matches);
    } else {
        // Text output mode
        cmd.arg("--color=auto");
        cmd.arg("--line-number");
        
        let status = cmd.status()
            .with_context(|| "failed to execute ripgrep (rg)")?;
        
        if !status.success() && status.code() != Some(1) {
            // Exit code 1 means no matches found, which is ok
            anyhow::bail!("ripgrep failed with exit code: {:?}", status.code());
        }
    }
    
    Ok(())
}

fn detect_workspace_root() -> PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Look for common workspace indicators going up the directory tree
    let mut dir = current.as_path();
    loop {
        for indicator in &[".git", "Cargo.toml", "package.json", ".gitignore", "pyproject.toml", "go.mod"] {
            if dir.join(indicator).exists() {
                return dir.to_path_buf();
            }
        }
        
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    
    // Fallback to current directory
    current
}

async fn cmd_agent(_globals: &GlobalOpts, _args: AgentArgs) -> anyhow::Result<()> { anyhow::bail!("agent command not yet implemented") }
async fn cmd_diff(globals: &GlobalOpts, command: DiffCommands) -> anyhow::Result<()> {
    match command {
        DiffCommands::Propose { instruction, file, files, provider } => {
            diff_propose(globals, instruction, file, files, provider).await
        }
        DiffCommands::Apply { file, yes } => {
            diff_apply(globals, file, yes).await
        }
    }
}

async fn diff_propose(
    globals: &GlobalOpts,
    instruction: String,
    single_file: Option<PathBuf>,
    multiple_files: Vec<PathBuf>,
    provider_override: Option<String>,
) -> anyhow::Result<()> {
    use crate::io::{read_file_to_string_async, filename_only, generate_unified_diff};
    use crate::llm::*;

    if instruction.trim().is_empty() {
        anyhow::bail!("Instruction cannot be empty");
    }

    // Determine which files to work with
    let target_files = if let Some(file) = single_file {
        vec![file]
    } else if !multiple_files.is_empty() {
        multiple_files
    } else {
        anyhow::bail!("Must specify either --file or --files");
    };

    // Load configuration
    dotenvy::dotenv().ok();
    
    let effective = config::resolve_effective_settings(
        globals.profile.as_deref(),
        provider_override.as_deref(),
        globals.model.as_deref(),
    )?;
    
    // Handle mock provider
    if effective.provider.to_lowercase() == "mock" {
        // Generate mock diffs for all target files
        let mut all_diffs = Vec::new();
        for file_path in &target_files {
            let original_content = if file_path.exists() {
                read_file_to_string_async(file_path).await.unwrap_or_default()
            } else {
                String::new()
            };
            
            let mock_new_content = if original_content.is_empty() {
                generate_mock_content(&instruction, &filename_only(file_path))
            } else {
                format!(
                    "{}\n// Mock diff for: {}\n// Instruction: {}",
                    original_content,
                    filename_only(file_path),
                    instruction
                )
            };
            
            let diff = generate_unified_diff(&original_content, &mock_new_content, &filename_only(file_path));
            all_diffs.push(diff);
        }
        
        // Output the diffs
        if globals.json {
            let json_response = serde_json::json!({
                "diffs": all_diffs,
                "target_files": target_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "instruction": instruction
            });
            println!("{}", serde_json::to_string_pretty(&json_response)?);
        } else {
            for diff in &all_diffs {
                print!("{}", diff);
            }
            
            if !all_diffs.is_empty() {
                println!("\nTo apply these changes, save the diff to a file and run:");
                println!("  sw diff apply --file <diff_file>");
            }
        }
        return Ok(());
    }
    
    let registry = ProviderRegistry::new()?;
    
    // Process each file separately to generate individual diffs
    let mut all_diffs = Vec::new();
    
    for file_path in &target_files {
        // Read existing content or use empty string for new files
        let original_content = if file_path.exists() {
            read_file_to_string_async(file_path).await.unwrap_or_default()
        } else {
            String::new()
        };

        // Build context for this specific file
        let mut context_parts = vec![
            format!("Task: {}", instruction),
            format!("Target file: {}", filename_only(file_path)),
            "".to_string(),
        ];

        if !original_content.is_empty() {
            context_parts.push("Current file content:".to_string());
            context_parts.push("```".to_string());
            context_parts.push(original_content.clone());
            context_parts.push("```".to_string());
            context_parts.push("".to_string());
            context_parts.push("Please provide the complete updated file content that implements the requested changes.".to_string());
        } else {
            context_parts.push("This is a new file.".to_string());
            context_parts.push("Please provide the complete file content that implements the requested functionality.".to_string());
        }

        context_parts.push("Focus on creating functional, well-structured code that fulfills the requirements.".to_string());

        let prompt = context_parts.join("\n");

        // Create LLM request
        let api_base = resolve_api_base_for_provider(&effective.provider);
        let request = LlmRequest {
            model: effective.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            stream: false, // Don't stream for diff generation
            api_base,
        };

        // Get the generated content
        let response = registry.get(&effective.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", effective.provider))?
            .send(request).await?;

        // Clean up the response content (remove markdown code blocks if present)
        let new_content = clean_generated_code(&response.content);

        // Generate unified diff
        let diff = generate_unified_diff(&original_content, &new_content, &filename_only(file_path));
        all_diffs.push(diff);
    }

    // Output the diffs
    if globals.json {
        let json_response = serde_json::json!({
            "diffs": all_diffs,
            "target_files": target_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "instruction": instruction
        });
        println!("{}", serde_json::to_string_pretty(&json_response)?);
    } else {
        for diff in &all_diffs {
            print!("{}", diff);
        }
        
        if !all_diffs.is_empty() {
            println!("\nTo apply these changes, save the diff to a file and run:");
            println!("  sw diff apply --file <diff_file>");
        }
    }

    Ok(())
}

/// Clean up generated code by removing markdown code blocks and extra formatting
fn clean_generated_code(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    
    // Remove leading and trailing code block markers
    if let Some(first) = lines.first() {
        if first.trim().starts_with("```") {
            lines.remove(0);
        }
    }
    
    if let Some(last) = lines.last() {
        if last.trim() == "```" {
            lines.pop();
        }
    }
    
    lines.join("\n")
}

async fn cmd_generate(
    globals: &GlobalOpts,
    instruction: String,
    single_file: Option<PathBuf>,
    multiple_files: Vec<PathBuf>,
    provider_override: Option<String>,
) -> anyhow::Result<()> {
    use crate::io::{read_file_to_string_async, filename_only, write_file_async};
    use crate::llm::*;

    if instruction.trim().is_empty() {
        anyhow::bail!("Instruction cannot be empty");
    }

    // Determine which files to work with
    let target_files = if let Some(file) = single_file {
        vec![file]
    } else if !multiple_files.is_empty() {
        multiple_files
    } else {
        anyhow::bail!("Must specify either --file or --files");
    };

    // Load configuration
    dotenvy::dotenv().ok();
    
    let effective = config::resolve_effective_settings(
        globals.profile.as_deref(),
        provider_override.as_deref(),
        globals.model.as_deref(),
    )?;
    
    // Handle mock provider
    if effective.provider.to_lowercase() == "mock" {
        // Generate mock content for all target files
        for file_path in &target_files {
            let mock_content = generate_mock_content(&instruction, &filename_only(file_path));
            write_file_async(file_path, &mock_content).await
                .with_context(|| format!("Writing mock content to {}", file_path.display()))?;
        }
        
        if globals.json {
            let json_response = serde_json::json!({
                "generated_content": format!("Mock content for {}", instruction),
                "target_files": target_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "instruction": instruction
            });
            println!("{}", serde_json::to_string_pretty(&json_response)?);
        } else {
            println!("Mock content generated for: {}", 
                target_files.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "));
        }
        return Ok(());
    }
    
    let registry = ProviderRegistry::new()?;
    
    // Build context with existing file contents
    let mut context_parts = vec![
        format!("Task: {}", instruction),
        "".to_string(),
        "Please generate code according to the instruction above.".to_string(),
    ];

    // Add existing file contents for context
    for file_path in &target_files {
        if file_path.exists() {
            match read_file_to_string_async(file_path).await {
                Ok(content) => {
                    context_parts.push(format!("Current content of {}:", filename_only(file_path)));
                    context_parts.push("```".to_string());
                    context_parts.push(content);
                    context_parts.push("```".to_string());
                    context_parts.push("".to_string());
                }
                Err(_) => {
                    context_parts.push(format!("File {} will be created as new.", filename_only(file_path)));
                }
            }
        } else {
            context_parts.push(format!("File {} will be created as new.", filename_only(file_path)));
        }
    }

    context_parts.push("Please provide the complete file content that should be generated, not a diff. Focus on creating functional, well-structured code that fulfills the requirements.".to_string());

    let prompt = context_parts.join("\n");

    // Create LLM request
    let api_base = resolve_api_base_for_provider(&effective.provider);
    let request = LlmRequest {
        model: effective.model.clone(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        stream: !globals.json,
        api_base,
    };

    // Send request to LLM
    if request.stream && !globals.json {
        // Stream the response for interactive use
        let mut stream = registry.get(&effective.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", effective.provider))?
            .send_stream(request).await?;
        
        let mut full_response = String::new();
        use futures_util::StreamExt;
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    print!("{}", chunk);
                    full_response.push_str(&chunk);
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                Err(e) => eprintln!("Stream error: {}", e),
            }
        }
        println!(); // New line after streaming

        // Clean up the response and write to files
        let clean_content = clean_generated_code(&full_response);
        for file_path in &target_files {
            write_file_async(file_path, &clean_content).await
                .with_context(|| format!("Writing generated content to {}", file_path.display()))?;
        }

        if !globals.json {
            println!("Generated content written to: {}", 
                target_files.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "));
        }
    } else {
        // Non-streaming response
        let response = registry.get(&effective.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", effective.provider))?
            .send(request).await?;

        let clean_content = clean_generated_code(&response.content);

        if globals.json {
            let json_response = serde_json::json!({
                "generated_content": clean_content,
                "target_files": target_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "instruction": instruction,
                "usage": response.usage
            });
            println!("{}", serde_json::to_string_pretty(&json_response)?);
        } else {
            println!("{}", clean_content);
        }

        // Write generated content to files
        for file_path in &target_files {
            write_file_async(file_path, &clean_content).await
                .with_context(|| format!("Writing generated content to {}", file_path.display()))?;
        }

        if !globals.json {
            println!("Generated content written to: {}", 
                target_files.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "));
        }
    }

    Ok(())
}

async fn diff_apply(
    _globals: &GlobalOpts,
    diff_file: PathBuf,
    auto_yes: bool,
) -> anyhow::Result<()> {
    use crate::io::{read_diff_file_async, apply_diff_to_content, backup_file_async, write_file_async, read_file_to_string_async};
    use std::io::{IsTerminal, Write};

    // Read the diff file
    let diff_content = read_diff_file_async(&diff_file).await?;
    
    if diff_content.trim().is_empty() {
        anyhow::bail!("Diff file is empty");
    }

    // Parse the diff to find target files (this is a simplified implementation)
    let target_files = parse_diff_target_files(&diff_content)?;
    
    if target_files.is_empty() {
        anyhow::bail!("No target files found in diff");
    }

    // Show what will be changed
    println!("Diff will be applied to the following files:");
    for file in &target_files {
        println!("  - {}", file.display());
    }
    println!();

    // Confirmation prompt unless --yes is specified
    if !auto_yes {
        if std::io::stdin().is_terminal() {
            print!("Apply these changes? [y/N]: ");
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().to_lowercase().starts_with('y') {
                println!("Operation cancelled.");
                return Ok(());
            }
        } else {
            anyhow::bail!("approval required");
        }
    }

    // Apply the diff to each target file
    for file_path in target_files {
        // Create backup
        let _backup_path = backup_file_async(&file_path).await?;
        
        // Read existing content or use empty string for new files
        let original_content = if file_path.exists() {
            read_file_to_string_async(&file_path).await?
        } else {
            String::new()
        };

        // Apply diff
        let new_content = apply_diff_to_content(&original_content, &diff_content)?;
        
        // Write the modified content
        write_file_async(&file_path, &new_content).await
            .with_context(|| format!("Applying diff to {}", file_path.display()))?;
        
        println!("Applied diff to: {}", file_path.display());
    }

    Ok(())
}

fn parse_diff_target_files(diff_content: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    for line in diff_content.lines() {
        if line.starts_with("+++") {
            // Extract filename from "+++ b/filename" or "+++ filename"
            let file_line = line.strip_prefix("+++").unwrap().trim();
            let filename = if file_line.starts_with("b/") {
                file_line.strip_prefix("b/").unwrap()
            } else {
                file_line
            };
            
            if !filename.is_empty() && filename != "/dev/null" {
                files.push(PathBuf::from(filename));
            }
        }
    }
    
    Ok(files)
}

/// Generate mock content based on instruction and filename for testing
fn generate_mock_content(instruction: &str, filename: &str) -> String {
    let instruction_lower = instruction.to_lowercase();
    
    // Determine file type from extension
    let file_ext = filename.split('.').last().unwrap_or("");
    
    let mut content = format!(
        "// Mock generated code for: {}\n// Instruction: {}\n\n",
        filename, instruction
    );
    
    match file_ext {
        "js" => {
            if instruction_lower.contains("express") || instruction_lower.contains("api") {
                content.push_str(&format!(
                    "const express = require('express');\nconst jwt = require('jsonwebtoken');\nconst router = express.Router();\n\n"
                ));
                
                if instruction_lower.contains("auth") {
                    content.push_str(&format!(
                        "router.post('/auth/login', (req, res) => {{\n    // Mock authentication endpoint\n    const token = jwt.sign({{ userId: 1 }}, 'secret');\n    res.json({{ token }});\n}});\n\n"
                    ));
                }
            } else if instruction_lower.contains("test") || instruction_lower.contains("jest") || filename.contains("test") {
                content.push_str(&format!(
                    "const {{ describe, it, expect }} = require('@jest/globals');\nconst UserService = require('./userService');\n\ndescribe('UserService Tests', () => {{\n    it('should createUser successfully', () => {{\n        const service = new UserService();\n        expect(service.createUser).toBeDefined();\n    }});\n    \n    it('should getUserById', () => {{\n        const service = new UserService();\n        expect(service.getUserById).toBeDefined();\n    }});\n    \n    it('should updateUser', () => {{\n        const service = new UserService();\n        expect(service.updateUser).toBeDefined();\n    }});\n    \n    it('should deleteUser', () => {{\n        const service = new UserService();\n        expect(service.deleteUser).toBeDefined();\n    }});\n}});\n"
                ));
            } else if instruction_lower.contains("refactor") || instruction_lower.contains("modern") || instruction_lower.contains("es6") {
                content.push_str(&format!(
                    "const processUsers = (users) => {{\n    return users\n        .filter(user => user.active === true)\n        .map(user => ({{ \n            id: user.id,\n            name: user.name,\n            email: user.email\n        }}));\n}};\n\nconst modernExample = () => {{\n    const data = [1, 2, 3, 4, 5];\n    const filtered = data.filter(n => n > 2);\n    const mapped = filtered.map(n => n * 2);\n    return mapped;\n}};\n"
                ));
            } else {
                content.push_str("const mockExample = () => {\n    console.log('Mock implementation');\n};\n");
            }
        },
        "md" => {
            content.push_str(&format!(
                "# {}\n\n## Installation\n\n```bash\nnpm install\n```\n\n## API Documentation\n\nMock API documentation generated.\n",
                filename.replace(".md", "").replace("_", " ").replace("-", " ")
            ));
        },
        "py" => {
            content.push_str("def mock_function():\n    \"\"\"Mock Python function\"\"\"\n    pass\n");
        },
        _ => {
            content.push_str("// Mock content\nfunction mockExample() {\n    console.log('Mock implementation');\n}\n");
        }
    }
    
    content.push_str(&format!("\nmodule.exports = {{ mockExample }};\n"));
    content
}

async fn cmd_files(globals: &GlobalOpts, command: FilesCommands) -> anyhow::Result<()> {
    match command {
        FilesCommands::List { 
            path, 
            recursive, 
            no_git, 
            include_ext, 
            exclude_ext, 
            include_pattern, 
            exclude_pattern 
        } => {
            use crate::io::batch::{FilePattern, find_files};
            
            // Build file pattern
            let mut pattern = FilePattern::new();
            
            if let Some(exts) = include_ext {
                for ext in exts.split(',') {
                    pattern = pattern.include_extension(ext.trim());
                }
            }
            
            if let Some(exts) = exclude_ext {
                for ext in exts.split(',') {
                    pattern = pattern.exclude_extension(ext.trim());
                }
            }
            
            if let Some(pat) = include_pattern {
                pattern = pattern.include_pattern(pat);
            }
            
            if let Some(pat) = exclude_pattern {
                pattern = pattern.exclude_pattern(pat);
            }
            
            // Find files (git-aware by default, disabled if --no-git flag is set)
            let git_aware_enabled = !no_git;
            let files = find_files(&path, &pattern, recursive, git_aware_enabled).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "files": files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                    "count": files.len(),
                    "git_aware": git_aware_enabled,
                    "recursive": recursive
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Found {} files:", files.len());
                for file in files {
                    println!("  {}", file.display());
                }
            }
        }
        FilesCommands::GitRoot { path } => {
            use crate::io::git::find_git_root;
            
            if let Some(git_root) = find_git_root(&path) {
                if globals.json {
                    let json_response = serde_json::json!({
                        "git_root": git_root.display().to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("Git root: {}", git_root.display());
                }
            } else {
                if globals.json {
                    let json_response = serde_json::json!({
                        "git_root": null,
                        "error": "No git repository found"
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("No git repository found starting from {}", path.display());
                }
            }
        }
        
        FilesCommands::Analyze { 
            path, 
            recursive, 
            include_ext, 
            exclude_ext, 
            detailed, 
            dependencies 
        } => {
            use crate::io::analysis::{analyze_directory, FileAnalysis, generate_dependency_graph};
            use crate::io::batch::FilePattern;
            
            // Build file pattern for analysis
            let mut pattern = FilePattern::new();
            
            if let Some(extensions) = include_ext {
                let exts: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
                for ext in exts {
                    pattern = pattern.include_extension(ext);
                }
            }
            
            if let Some(extensions) = exclude_ext {
                let exts: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
                for ext in exts {
                    pattern = pattern.exclude_extension(ext);
                }
            }
            
            // Analyze files
            let analyses = if path.is_file() {
                vec![FileAnalysis::analyze_file(&path).await?]
            } else {
                analyze_directory(&path, recursive, Some(&pattern)).await?
            };
            
            if globals.json {
                let mut json_data = serde_json::json!({
                    "path": path.display().to_string(),
                    "total_files": analyses.len(),
                    "analyses": []
                });
                
                if dependencies {
                    let dep_graph = generate_dependency_graph(&analyses);
                    json_data["dependency_graph"] = serde_json::to_value(dep_graph)?;
                }
                
                if detailed {
                    json_data["analyses"] = serde_json::to_value(&analyses)?;
                } else {
                    let summaries: Vec<_> = analyses.iter().map(|a| serde_json::json!({
                        "file": a.file_path.display().to_string(),
                        "language": a.language,
                        "type": format!("{:?}", a.file_type),
                        "lines": a.lines_of_code,
                        "functions": a.functions.len(),
                        "classes": a.classes.len(),
                        "imports": a.imports.len(),
                        "todos": a.todos.len(),
                        "complexity": a.complexity.cyclomatic_complexity
                    })).collect();
                    json_data["analyses"] = serde_json::to_value(summaries)?;
                }
                
                println!("{}", serde_json::to_string_pretty(&json_data)?);
            } else {
                println!("File Analysis Report");
                println!("===================");
                println!("Path: {}", path.display());
                println!("Files analyzed: {}", analyses.len());
                println!();
                
                // Summary statistics
                let total_lines: usize = analyses.iter().map(|a| a.lines_of_code).sum();
                let total_functions: usize = analyses.iter().map(|a| a.functions.len()).sum();
                let total_classes: usize = analyses.iter().map(|a| a.classes.len()).sum();
                let total_todos: usize = analyses.iter().map(|a| a.todos.len()).sum();
                let avg_complexity: f64 = if analyses.is_empty() { 0.0 } else {
                    analyses.iter().map(|a| a.complexity.cyclomatic_complexity as f64).sum::<f64>() / analyses.len() as f64
                };
                
                println!("Summary:");
                println!("  Total lines of code: {}", total_lines);
                println!("  Total functions: {}", total_functions);
                println!("  Total classes: {}", total_classes);
                println!("  Total TODOs: {}", total_todos);
                println!("  Average complexity: {:.1}", avg_complexity);
                println!();
                
                // Language breakdown
                let mut languages = std::collections::HashMap::new();
                for analysis in &analyses {
                    *languages.entry(&analysis.language).or_insert(0) += 1;
                }
                
                println!("Languages:");
                for (lang, count) in languages {
                    println!("  {}: {} files", lang, count);
                }
                println!();
                
                if dependencies {
                    let dep_graph = generate_dependency_graph(&analyses);
                    println!("Dependency Graph:");
                    for (file, deps) in dep_graph {
                        if !deps.is_empty() {
                            println!("  {} depends on:", file);
                            for dep in deps {
                                println!("    - {}", dep);
                            }
                        }
                    }
                    println!();
                }
                
                if detailed {
                    println!("Detailed Analysis:");
                    for analysis in &analyses {
                        println!("  {}", analysis.summary());
                        
                        if !analysis.functions.is_empty() {
                            println!("    Functions:");
                            for func in &analysis.functions {
                                println!("      - {} (line {}, {} params, {})", 
                                    func.name, func.line_start, func.parameters.len(),
                                    if func.is_async { "async" } else { "sync" });
                            }
                        }
                        
                        if !analysis.classes.is_empty() {
                            println!("    Classes:");
                            for class in &analysis.classes {
                                println!("      - {} (line {})", class.name, class.line_start);
                            }
                        }
                        
                        if !analysis.todos.is_empty() {
                            println!("    TODOs:");
                            for todo in &analysis.todos {
                                println!("      - {:?}: {} (line {})", todo.todo_type, todo.content, todo.line);
                            }
                        }
                        
                        println!();
                    }
                } else {
                    // Just show file summaries
                    for analysis in &analyses {
                        println!("  {}", analysis.summary());
                    }
                }
            }
        }
        
        FilesCommands::Compare { source, target, content, ignore_time, exclude } => {
            use crate::io::sync::{compare_directories, SyncOptions};
            
            let mut options = SyncOptions::default();
            options.include_content = content;
            options.ignore_timestamps = ignore_time;
            
            if let Some(exclude_patterns) = exclude {
                options.exclude_patterns = exclude_patterns.split(',').map(|s| s.trim().to_string()).collect();
            }
            
            let diffs = compare_directories(&source, &target, &options).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "source": source.display().to_string(),
                    "target": target.display().to_string(),
                    "differences": diffs,
                    "total_differences": diffs.len()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Directory Comparison");
                println!("==================");
                println!("Source: {}", source.display());
                println!("Target: {}", target.display());
                println!();
                
                let mut added = 0;
                let mut modified = 0;
                let mut deleted = 0;
                let mut identical = 0;
                
                for diff in &diffs {
                    match diff.status {
                        crate::io::sync::DiffStatus::Added => {
                            println!("+ {} ({})", diff.path.display(), format_size(diff.size_new.unwrap_or(0)));
                            added += 1;
                        }
                        crate::io::sync::DiffStatus::Deleted => {
                            println!("- {} ({})", diff.path.display(), format_size(diff.size_old.unwrap_or(0)));
                            deleted += 1;
                        }
                        crate::io::sync::DiffStatus::Modified => {
                            println!("M {} ({} -> {})", 
                                diff.path.display(), 
                                format_size(diff.size_old.unwrap_or(0)),
                                format_size(diff.size_new.unwrap_or(0))
                            );
                            modified += 1;
                        }
                        crate::io::sync::DiffStatus::Renamed { ref old_path } => {
                            println!("R {} -> {}", old_path.display(), diff.path.display());
                            modified += 1;
                        }
                        crate::io::sync::DiffStatus::Identical => {
                            identical += 1;
                        }
                    }
                }
                
                println!();
                println!("Summary:");
                println!("  Added: {}", added);
                println!("  Modified: {}", modified);
                println!("  Deleted: {}", deleted);
                println!("  Identical: {}", identical);
                println!("  Total: {}", diffs.len());
            }
        }
        
        FilesCommands::Sync { source, target, dry_run, content, exclude } => {
            use crate::io::sync::{compare_directories, sync_files, SyncOptions};
            
            let mut options = SyncOptions::default();
            options.include_content = content;
            
            if let Some(exclude_patterns) = exclude {
                options.exclude_patterns = exclude_patterns.split(',').map(|s| s.trim().to_string()).collect();
            }
            
            let diffs = compare_directories(&source, &target, &options).await?;
            let sync_diffs: Vec<_> = diffs.iter()
                .filter(|d| !matches!(d.status, crate::io::sync::DiffStatus::Identical))
                .collect();
            
            if sync_diffs.is_empty() {
                if globals.json {
                    let json_response = serde_json::json!({
                        "source": source.display().to_string(),
                        "target": target.display().to_string(),
                        "synced_files": [],
                        "dry_run": dry_run,
                        "message": "Directories are already in sync"
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("✅ Directories are already in sync!");
                }
                return Ok(());
            }
            
            if dry_run {
                if globals.json {
                    let json_response = serde_json::json!({
                        "source": source.display().to_string(),
                        "target": target.display().to_string(),
                        "changes_needed": sync_diffs,
                        "dry_run": true
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("🔍 Dry Run - Changes that would be made:");
                    println!("========================================");
                    
                    for diff in &sync_diffs {
                        match diff.status {
                            crate::io::sync::DiffStatus::Added => {
                                println!("+ Would copy: {}", diff.path.display());
                            }
                            crate::io::sync::DiffStatus::Modified => {
                                println!("M Would update: {}", diff.path.display());
                            }
                            crate::io::sync::DiffStatus::Deleted => {
                                println!("- Would delete: {}", diff.path.display());
                            }
                            crate::io::sync::DiffStatus::Renamed { ref old_path } => {
                                println!("R Would rename: {} -> {}", old_path.display(), diff.path.display());
                            }
                            _ => {}
                        }
                    }
                    
                    println!("\n{} changes would be made", sync_diffs.len());
                    println!("Run without --dry-run to apply changes");
                }
            } else {
                let owned_diffs: Vec<_> = sync_diffs.iter().map(|d| (*d).clone()).collect();
                let synced_files = sync_files(&source, &target, &owned_diffs, false).await?;
                
                if globals.json {
                    let json_response = serde_json::json!({
                        "source": source.display().to_string(),
                        "target": target.display().to_string(),
                        "synced_files": synced_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                        "count": synced_files.len(),
                        "dry_run": false
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("🔄 Synchronizing directories...");
                    println!("===============================");
                    
                    for file in &synced_files {
                        println!("✅ Synced: {}", file.display());
                    }
                    
                    println!("\n🎉 Successfully synced {} files!", synced_files.len());
                }
            }
        }
        
        FilesCommands::Duplicates { path, recursive } => {
            use crate::io::sync::find_duplicate_files;
            
            let duplicate_groups = find_duplicate_files(&path, recursive).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "path": path.display().to_string(),
                    "duplicate_groups": duplicate_groups.iter().map(|group| {
                        group.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()
                    }).collect::<Vec<_>>(),
                    "groups_count": duplicate_groups.len(),
                    "total_duplicates": duplicate_groups.iter().map(|g| g.len()).sum::<usize>()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Duplicate Files Report");
                println!("=====================");
                println!("Path: {}", path.display());
                println!();
                
                if duplicate_groups.is_empty() {
                    println!("✅ No duplicate files found!");
                } else {
                    let total_duplicates: usize = duplicate_groups.iter().map(|g| g.len()).sum();
                    println!("Found {} groups of duplicate files ({} total files):", 
                        duplicate_groups.len(), total_duplicates);
                    println!();
                    
                    for (i, group) in duplicate_groups.iter().enumerate() {
                        println!("Group {} ({} files):", i + 1, group.len());
                        for file in group {
                            let metadata = std::fs::metadata(file)?;
                            println!("  📄 {} ({})", file.display(), format_size(metadata.len()));
                        }
                        println!();
                    }
                    
                    let space_wasted: u64 = duplicate_groups.iter()
                        .flat_map(|group| group.iter().skip(1))
                        .map(|file| std::fs::metadata(file).map(|m| m.len()).unwrap_or(0))
                        .sum();
                    
                    println!("💾 Potential space savings: {}", format_size(space_wasted));
                }
            }
        }
        
        FilesCommands::Search { 
            pattern, 
            path, 
            case_sensitive, 
            regex, 
            fuzzy, 
            semantic, 
            whole_word, 
            context, 
            types, 
            max_matches 
        } => {
            use crate::io::search::{search_files, SearchOptions};
            
            let mut options = SearchOptions::default();
            options.pattern = pattern;
            options.case_sensitive = case_sensitive;
            options.regex = regex;
            options.fuzzy = fuzzy;
            options.semantic = semantic;
            options.whole_word = whole_word;
            options.context_lines = context;
            options.max_matches_per_file = max_matches;
            
            if let Some(file_types) = types {
                options.file_types = file_types.split(',').map(|s| s.trim().to_string()).collect();
            }
            
            let results = search_files(&path, &options).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "pattern": options.pattern,
                    "path": path.display().to_string(),
                    "total_files": results.len(),
                    "total_matches": results.iter().map(|r| r.total_matches).sum::<usize>(),
                    "results": results
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Advanced Search Results");
                println!("======================");
                println!("Pattern: {}", options.pattern);
                println!("Path: {}", path.display());
                println!();
                
                if results.is_empty() {
                    println!("❌ No matches found");
                } else {
                    let total_matches: usize = results.iter().map(|r| r.total_matches).sum();
                    println!("Found {} matches in {} files:", total_matches, results.len());
                    println!();
                    
                    for (file_idx, result) in results.iter().enumerate() {
                        if file_idx > 0 {
                            println!();
                        }
                        
                        println!("📄 {} ({} matches, {})", 
                            result.file_path.display(), 
                            result.total_matches,
                            format_size(result.file_size)
                        );
                        
                        for (match_idx, search_match) in result.matches.iter().enumerate() {
                            if match_idx >= 5 && !globals.json { // Limit display for readability
                                println!("   ... {} more matches", result.matches.len() - match_idx);
                                break;
                            }
                            
                            // Show context before
                            for context_line in &search_match.context_before {
                                println!("   {} | {}", "   ".to_string(), context_line);
                            }
                            
                            // Show the match line with highlighting
                            let match_type_symbol = match search_match.match_type {
                                crate::io::search::MatchType::Exact => "=",
                                crate::io::search::MatchType::Regex => "~",
                                crate::io::search::MatchType::Fuzzy => "?",
                                crate::io::search::MatchType::FunctionName => "fn",
                                crate::io::search::MatchType::ClassName => "cls",
                                _ => "*",
                            };
                            
                            println!("   {:3} | {} [{}] \"{}\"", 
                                search_match.line_number, 
                                search_match.line_content.trim(),
                                match_type_symbol,
                                search_match.match_text
                            );
                            
                            // Show context after  
                            for context_line in &search_match.context_after {
                                println!("   {} | {}", "   ".to_string(), context_line);
                            }
                        }
                    }
                    
                    println!();
                    println!("Search Options:");
                    println!("  Case sensitive: {}", options.case_sensitive);
                    println!("  Regex: {}", options.regex);
                    println!("  Fuzzy: {}", options.fuzzy);
                    println!("  Semantic: {}", options.semantic);
                    println!("  Whole word: {}", options.whole_word);
                    println!("  Context lines: {}", options.context_lines);
                }
            }
        }
        
        FilesCommands::Replace { 
            pattern, 
            replace, 
            path, 
            dry_run, 
            case_sensitive, 
            regex, 
            types 
        } => {
            use crate::io::search::{search_and_replace, SearchOptions};
            
            let mut options = SearchOptions::default();
            options.pattern = pattern.clone();
            options.case_sensitive = case_sensitive;
            options.regex = regex;
            
            if let Some(file_types) = types {
                options.file_types = file_types.split(',').map(|s| s.trim().to_string()).collect();
            }
            
            let replaced_files = search_and_replace(&path, &pattern, &replace, &options, dry_run).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "pattern": pattern,
                    "replacement": replace,
                    "path": path.display().to_string(),
                    "dry_run": dry_run,
                    "files_affected": replaced_files.len(),
                    "total_replacements": replaced_files.iter().map(|(_, count)| count).sum::<usize>(),
                    "files": replaced_files.iter().map(|(path, count)| {
                        serde_json::json!({
                            "file": path.display().to_string(),
                            "replacements": count
                        })
                    }).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                if dry_run {
                    println!("🔍 Search and Replace (Dry Run)");
                } else {
                    println!("🔄 Search and Replace");
                }
                println!("================================");
                println!("Pattern: {}", pattern);
                println!("Replace: {}", replace);
                println!("Path: {}", path.display());
                println!();
                
                if replaced_files.is_empty() {
                    println!("❌ No matches found to replace");
                } else {
                    let total_replacements: usize = replaced_files.iter().map(|(_, count)| count).sum();
                    
                    if dry_run {
                        println!("Would replace {} occurrences in {} files:", total_replacements, replaced_files.len());
                    } else {
                        println!("✅ Replaced {} occurrences in {} files:", total_replacements, replaced_files.len());
                    }
                    println!();
                    
                    for (file_path, count) in &replaced_files {
                        if dry_run {
                            println!("  📄 {} ({} replacements would be made)", file_path.display(), count);
                        } else {
                            println!("  ✅ {} ({} replacements)", file_path.display(), count);
                        }
                    }
                    
                    if dry_run {
                        println!();
                        println!("Run without --dry-run to apply changes");
                    }
                }
            }
        }
        
        FilesCommands::Security { 
            path, 
            include_info,
            check_credentials,
            check_injection,
            check_crypto,
            check_paths,
            check_dependencies,
            check_configuration,
            types,
            high_only,
            min_risk
        } => {
            use crate::io::security::{scan_files_security, SecurityOptions, Severity};
            
            let mut options = SecurityOptions::default();
            options.include_info = include_info;
            options.check_credentials = check_credentials;
            options.check_injection = check_injection;
            options.check_crypto = check_crypto;
            options.check_paths = check_paths;
            options.check_dependencies = check_dependencies;
            options.check_configuration = check_configuration;
            
            if let Some(file_types) = types {
                options.file_types = file_types.split(',').map(|s| s.trim().to_string()).collect();
            }
            
            let mut reports = scan_files_security(&path, &options).await?;
            
            // Filter by minimum risk score if specified
            if let Some(min_score) = min_risk {
                reports.retain(|report| report.risk_score >= min_score);
            }
            
            // Filter to high severity and above if specified
            if high_only {
                reports.retain(|report| {
                    report.issues.iter().any(|issue| 
                        matches!(issue.severity, Severity::High | Severity::Critical)
                    )
                });
            }
            
            if globals.json {
                let json_response = serde_json::json!({
                    "scan_summary": {
                        "path": path.display().to_string(),
                        "total_files_scanned": reports.len(),
                        "total_issues": reports.iter().map(|r| r.issues.len()).sum::<usize>(),
                        "total_risk_score": reports.iter().map(|r| r.risk_score).sum::<u32>(),
                        "critical_issues": reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::Critical).count(),
                        "high_issues": reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::High).count(),
                        "medium_issues": reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::Medium).count(),
                        "low_issues": reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::Low).count(),
                    },
                    "reports": reports
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("🛡️ Security Vulnerability Scan");
                println!("==============================");
                println!("Path: {}", path.display());
                println!();
                
                if reports.is_empty() {
                    println!("✅ No security issues found or no files match criteria");
                } else {
                    let total_issues: usize = reports.iter().map(|r| r.issues.len()).sum();
                    let total_risk_score: u32 = reports.iter().map(|r| r.risk_score).sum();
                    
                    let critical_count = reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::Critical).count();
                    let high_count = reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::High).count();
                    let medium_count = reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::Medium).count();
                    let low_count = reports.iter().flat_map(|r| &r.issues).filter(|i| i.severity == Severity::Low).count();
                    
                    println!("📊 Summary:");
                    println!("  Files scanned: {}", reports.len());
                    println!("  Total issues: {}", total_issues);
                    println!("  Total risk score: {}", total_risk_score);
                    println!();
                    
                    println!("📈 Issues by severity:");
                    if critical_count > 0 { println!("  🚨 Critical: {}", critical_count); }
                    if high_count > 0 { println!("  ⚠️  High: {}", high_count); }
                    if medium_count > 0 { println!("  ⚡ Medium: {}", medium_count); }
                    if low_count > 0 { println!("  ℹ️  Low: {}", low_count); }
                    println!();
                    
                    // Show detailed reports for files with issues
                    for (file_idx, report) in reports.iter().enumerate() {
                        if report.issues.is_empty() {
                            continue;
                        }
                        
                        if file_idx > 0 {
                            println!();
                        }
                        
                        println!("📄 {} (Risk Score: {}, {} issues)", 
                            report.file_path.display(), 
                            report.risk_score,
                            report.issues.len()
                        );
                        
                        // Group issues by severity
                        let critical_issues: Vec<_> = report.issues.iter().filter(|i| i.severity == Severity::Critical).collect();
                        let high_issues: Vec<_> = report.issues.iter().filter(|i| i.severity == Severity::High).collect();
                        let medium_issues: Vec<_> = report.issues.iter().filter(|i| i.severity == Severity::Medium).collect();
                        let low_issues: Vec<_> = report.issues.iter().filter(|i| i.severity == Severity::Low).collect();
                        
                        // Show critical issues first
                        for issue in critical_issues.iter() {
                            println!("   🚨 CRITICAL (Line {}): {}", issue.line_number, issue.description);
                            if let Some(cwe) = &issue.cwe_id {
                                println!("      CWE: {} | Recommendation: {}", cwe, issue.recommendation);
                            }
                            if !globals.json && issue.line_content.len() < 120 {
                                println!("      Code: {}", issue.line_content.trim());
                            }
                        }
                        
                        // Show high issues
                        for issue in high_issues.iter() {
                            println!("   ⚠️  HIGH (Line {}): {}", issue.line_number, issue.description);
                            if let Some(cwe) = &issue.cwe_id {
                                println!("      CWE: {} | Recommendation: {}", cwe, issue.recommendation);
                            }
                        }
                        
                        // Show medium issues (limit display for readability)
                        for (idx, issue) in medium_issues.iter().enumerate() {
                            if idx >= 3 && !globals.json {
                                println!("   ⚡ ... {} more medium severity issues", medium_issues.len() - idx);
                                break;
                            }
                            println!("   ⚡ MEDIUM (Line {}): {}", issue.line_number, issue.description);
                        }
                        
                        // Show low issues (limit display for readability)
                        if !low_issues.is_empty() && !high_only {
                            if low_issues.len() <= 2 {
                                for issue in low_issues.iter() {
                                    println!("   ℹ️  LOW (Line {}): {}", issue.line_number, issue.description);
                                }
                            } else {
                                println!("   ℹ️  {} low severity issues (use --include-info for details)", low_issues.len());
                            }
                        }
                        
                        // Show recommendations
                        if !report.recommendations.is_empty() {
                            println!("   💡 Recommendations:");
                            for (idx, rec) in report.recommendations.iter().enumerate() {
                                if idx >= 2 && !globals.json {
                                    println!("      ... {} more recommendations", report.recommendations.len() - idx);
                                    break;
                                }
                                println!("      • {}", rec);
                            }
                        }
                    }
                    
                    println!();
                    println!("🔍 Scan Options:");
                    println!("  Credentials check: {}", options.check_credentials);
                    println!("  Injection check: {}", options.check_injection);
                    println!("  Cryptography check: {}", options.check_crypto);
                    println!("  Path traversal check: {}", options.check_paths);
                    println!("  Dependencies check: {}", options.check_dependencies);
                    println!("  Configuration check: {}", options.check_configuration);
                    
                    if total_risk_score > 200 {
                        println!();
                        println!("⚠️  HIGH RISK: Total risk score {} indicates significant security concerns", total_risk_score);
                        println!("   Consider immediate remediation of critical and high severity issues");
                    } else if total_risk_score > 100 {
                        println!();
                        println!("⚡ MEDIUM RISK: Total risk score {} indicates moderate security concerns", total_risk_score);
                        println!("   Review and address identified issues when possible");
                    } else if total_risk_score > 0 {
                        println!();
                        println!("ℹ️  LOW RISK: Total risk score {} indicates minor security concerns", total_risk_score);
                    } else {
                        println!();
                        println!("✅ No security issues detected");
                    }
                }
            }
        }
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

async fn cmd_checkpoint(globals: &GlobalOpts, command: CheckpointCommands) -> anyhow::Result<()> {
    use crate::io::checkpoint;
    
    match command {
        CheckpointCommands::Create { description, files } => {
            let checkpoint_path = checkpoint::create_auto_checkpoint(&files, &description).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "checkpoint_path": checkpoint_path.display().to_string(),
                    "files": files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                    "description": description
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Checkpoint created: {}", checkpoint_path.display());
                println!("Files saved: {}", files.len());
                for file in files {
                    println!("  {}", file.display());
                }
            }
        }
        CheckpointCommands::List => {
            let checkpoints = checkpoint::list_checkpoints().await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "checkpoints": checkpoints.iter().map(|cp| serde_json::json!({
                        "id": cp.id,
                        "timestamp": cp.timestamp,
                        "description": cp.description,
                        "files_count": cp.files.len()
                    })).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                if checkpoints.is_empty() {
                    println!("No checkpoints found");
                } else {
                    println!("Available checkpoints:");
                    for checkpoint in checkpoints {
                        let _datetime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(checkpoint.timestamp);
                        println!("  {} - {} ({} files)", 
                            checkpoint.id,
                            checkpoint.description,
                            checkpoint.files.len()
                        );
                    }
                }
            }
        }
        CheckpointCommands::Restore { id } => {
            let checkpoints = checkpoint::list_checkpoints().await?;
            
            if let Some(checkpoint) = checkpoints.into_iter().find(|cp| cp.id == id) {
                checkpoint.restore().await?;
                
                if globals.json {
                    let json_response = serde_json::json!({
                        "restored": true,
                        "checkpoint_id": id,
                        "files_restored": checkpoint.files.len()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_response)?);
                } else {
                    println!("Restored checkpoint: {}", id);
                    println!("Files restored: {}", checkpoint.files.len());
                    for file in checkpoint.files {
                        println!("  {}", file.path.display());
                    }
                }
            } else {
                anyhow::bail!("Checkpoint not found: {}", id);
            }
        }
    }
    Ok(())
}

async fn cmd_batch(globals: &GlobalOpts, command: BatchCommands) -> anyhow::Result<()> {
    use crate::io::batch::{FilePattern, find_files};
    use crate::io::checkpoint;
    
    match command {
        BatchCommands::Generate { 
            instruction, 
            path, 
            recursive, 
            include_ext, 
            exclude_ext, 
            provider, 
            checkpoint: create_checkpoint 
        } => {
            // Build file pattern
            let mut pattern = FilePattern::new();
            
            if let Some(exts) = include_ext {
                for ext in exts.split(',') {
                    pattern = pattern.include_extension(ext.trim());
                }
            }
            
            if let Some(exts) = exclude_ext {
                for ext in exts.split(',') {
                    pattern = pattern.exclude_extension(ext.trim());
                }
            }
            
            // Find target files
            let files = find_files(&path, &pattern, recursive, true).await?;
            
            if files.is_empty() {
                println!("No files found matching the criteria");
                return Ok(());
            }
            
            // Create checkpoint if requested
            let checkpoint_path = if create_checkpoint {
                Some(checkpoint::create_auto_checkpoint(&files, &format!("Before batch generate: {}", instruction)).await?)
            } else {
                None
            };
            
            // Process each file
            let mut results = Vec::new();
            for file in &files {
                let result = cmd_generate(
                    globals, 
                    instruction.clone(), 
                    Some(file.clone()), 
                    Vec::new(), 
                    provider.clone()
                ).await;
                results.push((file.clone(), result));
            }
            
            // Report results
            if globals.json {
                let json_response = serde_json::json!({
                    "processed_files": files.len(),
                    "checkpoint": checkpoint_path.as_ref().map(|p| p.display().to_string()),
                    "results": results.iter().map(|(file, result)| serde_json::json!({
                        "file": file.display().to_string(),
                        "success": result.is_ok(),
                        "error": result.as_ref().err().map(|e| e.to_string())
                    })).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Processed {} files", files.len());
                if let Some(cp_path) = checkpoint_path {
                    println!("Checkpoint created: {}", cp_path.display());
                }
                
                let successful = results.iter().filter(|(_, r)| r.is_ok()).count();
                let failed = results.iter().filter(|(_, r)| r.is_err()).count();
                
                println!("Success: {}, Failed: {}", successful, failed);
                
                if failed > 0 {
                    println!("Failed files:");
                    for (file, result) in results {
                        if let Err(e) = result {
                            println!("  {}: {}", file.display(), e);
                        }
                    }
                }
            }
        }
        BatchCommands::Transform { 
            instruction, 
            path, 
            recursive, 
            include_ext, 
            provider, 
            checkpoint: create_checkpoint 
        } => {
            // Similar to Generate but focuses on transforming existing files
            let mut pattern = FilePattern::new();
            
            if let Some(exts) = include_ext {
                for ext in exts.split(',') {
                    pattern = pattern.include_extension(ext.trim());
                }
            }
            
            let files = find_files(&path, &pattern, recursive, true).await?;
            
            if files.is_empty() {
                println!("No files found matching the criteria");
                return Ok(());
            }
            
            // Create checkpoint if requested
            let checkpoint_path = if create_checkpoint {
                Some(checkpoint::create_auto_checkpoint(&files, &format!("Before batch transform: {}", instruction)).await?)
            } else {
                None
            };
            
            // Use diff propose for safer transformations
            for file in &files {
                let _ = diff_propose(
                    globals, 
                    instruction.clone(), 
                    Some(file.clone()), 
                    Vec::new(),
                    provider.clone()
                ).await;
            }
            
            if !globals.json {
                println!("Generated diffs for {} files", files.len());
                if let Some(cp_path) = checkpoint_path {
                    println!("Checkpoint created: {}", cp_path.display());
                }
                println!("Review the diffs and apply them using 'sw diff apply --file <diff_file>'");
            }
        }
    }
    Ok(())
}

async fn cmd_template(globals: &GlobalOpts, command: TemplateCommands) -> anyhow::Result<()> {
    use crate::io::templates::{list_templates, generate_from_template};
    use std::collections::HashMap;
    
    match command {
        TemplateCommands::List => {
            let templates = list_templates().await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "templates": templates
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("Available Templates:");
                println!("==================");
                
                for template in templates {
                    println!("\n📋 {}", template.name);
                    println!("   Language: {}", template.language);
                    println!("   Description: {}", template.description);
                    
                    if !template.variables.is_empty() {
                        println!("   Variables:");
                        for var in &template.variables {
                            let required = if var.required { "*" } else { "" };
                            let default = var.default_value.as_ref()
                                .map(|d| format!(" (default: {})", d))
                                .unwrap_or_default();
                            println!("     - {}{}: {}{}", var.name, required, var.description, default);
                        }
                    }
                    
                    if !template.dependencies.is_empty() {
                        println!("   Dependencies: {}", template.dependencies.join(", "));
                    }
                    
                    if !template.scripts.is_empty() {
                        let script_names: Vec<String> = template.scripts.keys().map(|k| k.clone()).collect();
                        println!("   Scripts: {}", script_names.join(", "));
                    }
                    
                    println!("   Files: {} files", template.files.len());
                }
                
                println!("\nUsage:");
                println!("  sw template generate --template <name> --name <project-name> [--var key=value]");
                println!("  * = required variable");
            }
        }
        
        TemplateCommands::Generate { template, output, name, author, var } => {
            // Parse variables
            let mut variables = HashMap::new();
            for var_str in var {
                if let Some((key, value)) = var_str.split_once('=') {
                    variables.insert(key.to_string(), value.to_string());
                } else {
                    return Err(anyhow::anyhow!("Invalid variable format: {}. Use key=value", var_str));
                }
            }
            
            // Generate from template
            let created_files = generate_from_template(&template, &output, variables, &name, &author).await?;
            
            if globals.json {
                let json_response = serde_json::json!({
                    "template": template,
                    "output_directory": output.display().to_string(),
                    "project_name": name,
                    "files_created": created_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                    "count": created_files.len()
                });
                println!("{}", serde_json::to_string_pretty(&json_response)?);
            } else {
                println!("🚀 Generated project from template '{}'", template);
                println!("📁 Output directory: {}", output.display());
                println!("📝 Project name: {}", name);
                println!("👤 Author: {}", author);
                println!("\n📋 Created files:");
                
                for file in &created_files {
                    println!("  ✅ {}", file.display());
                }
                
                println!("\n🎉 Successfully created {} files!", created_files.len());
                println!("\nNext steps:");
                println!("  cd {}", output.display());
                
                // Show relevant next steps based on template
                match template.as_str() {
                    "rust-cli" => {
                        println!("  cargo build");
                        println!("  cargo run -- hello World");
                    }
                    "node-express" => {
                        println!("  npm install");
                        println!("  npm run dev");
                    }
                    "python-fastapi" => {
                        println!("  pip install -r requirements.txt");
                        println!("  python main.py");
                    }
                    "react-component" => {
                        println!("  # Import the component in your React app");
                    }
                    "typescript-library" => {
                        println!("  npm install");
                        println!("  npm run build");
                    }
                    _ => {
                        println!("  # Check the README.md for setup instructions");
                    }
                }
            }
        }
    }
    
    Ok(())
}

