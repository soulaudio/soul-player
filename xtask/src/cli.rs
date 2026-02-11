use clap::{Parser, Subcommand};

/// Soul Player development tasks
#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Soul Player development task automation", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Quality checks (formatting, linting, tests)
    #[command(subcommand)]
    Check(CheckCommands),

    /// Build commands
    #[command(subcommand)]
    Build(BuildCommands),

    /// Test commands
    #[command(subcommand)]
    Test(TestCommands),

    /// Setup commands (first-time setup, dependencies)
    #[command(subcommand)]
    Setup(SetupCommands),

    /// Cleanup commands
    #[command(subcommand)]
    Clean(CleanCommands),

    /// Version management
    #[command(subcommand)]
    Version(VersionCommands),

    /// Development servers
    #[command(subcommand)]
    Dev(DevCommands),

    /// CI/CD utilities
    #[command(subcommand)]
    Ci(CiCommands),
}

// ============================================================================
// Check Commands
// ============================================================================

#[derive(Subcommand)]
pub enum CheckCommands {
    /// Run pre-commit checks (fmt, clippy, test, TypeScript, lint)
    Precommit,

    /// Run CI-optimized checks
    Ci,

    /// Check Rust formatting
    Fmt {
        /// Auto-fix formatting issues
        #[arg(long)]
        fix: bool,
    },

    /// Run Clippy lints
    Clippy {
        /// Auto-fix Clippy suggestions
        #[arg(long)]
        fix: bool,
    },

    /// Run Rust tests
    Test {
        /// Run specific package tests
        #[arg(long, short)]
        package: Option<String>,

        /// Pass additional arguments to cargo test
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Check TypeScript (all workspaces)
    Typescript {
        /// Check specific workspace
        #[arg(long, short)]
        workspace: Option<String>,
    },

    /// Run ESLint (all workspaces)
    Lint {
        /// Auto-fix lint issues
        #[arg(long)]
        fix: bool,

        /// Lint specific workspace
        #[arg(long, short)]
        workspace: Option<String>,
    },
}

// ============================================================================
// Build Commands
// ============================================================================

#[derive(Subcommand)]
pub enum BuildCommands {
    /// Build desktop Tauri app
    Desktop {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
    },

    /// Build mobile Tauri app
    Mobile {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,

        /// Target platform (ios, android)
        #[arg(long)]
        platform: Option<String>,
    },

    /// Build WASM modules
    Wasm {
        /// Watch for changes and rebuild
        #[arg(long, short)]
        watch: bool,

        /// Build in release mode
        #[arg(long, short)]
        release: bool,
    },

    /// Build marketing site
    Marketing {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
    },

    /// Build web app
    Web {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
    },

    /// Build all targets
    All {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
    },
}

// ============================================================================
// Test Commands
// ============================================================================

#[derive(Subcommand)]
pub enum TestCommands {
    /// Audio testing commands
    #[command(subcommand)]
    Audio(AudioCommands),

    /// Import/re-import testing commands
    #[command(subcommand)]
    Import(ImportCommands),

    /// Cache invalidation testing commands
    #[command(subcommand)]
    Cache(CacheCommands),

    /// Run unit tests
    Unit {
        /// Run specific package tests
        #[arg(long, short)]
        package: Option<String>,

        /// Pass additional arguments to cargo test
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Run integration tests
    Integration,

    /// Run WebDriver E2E tests
    E2e {
        /// Run specific test suite
        #[arg(long)]
        suite: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum AudioCommands {
    /// Run audio E2E tests
    E2e {
        /// Run tests in CI mode (shorter timeouts, less verbose)
        #[arg(long)]
        ci: bool,

        /// Skip virtual device check (for debugging)
        #[arg(long)]
        skip_device_check: bool,

        /// Run only initialization tests
        #[arg(long)]
        init_only: bool,

        /// Run only stutter tests
        #[arg(long)]
        stutter_only: bool,

        /// Export metrics to JSON file
        #[arg(long, value_name = "FILE")]
        export_metrics: Option<String>,
    },

    /// List available audio devices
    ListDevices {
        /// Show detailed device information
        #[arg(long, short)]
        verbose: bool,

        /// Filter by device name (case-insensitive)
        #[arg(long, short)]
        filter: Option<String>,
    },

    /// Generate test audio assets
    GenerateAssets {
        /// Output directory for test assets
        #[arg(long, short, default_value = "tests/assets")]
        output: String,

        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },

    /// CI-friendly audio E2E tests
    Ci {
        /// Maximum test duration in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,

        /// Export metrics to JSON file
        #[arg(long, value_name = "FILE")]
        export_metrics: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Run import/re-import E2E tests
    E2e {
        /// Run tests in CI mode (shorter timeouts)
        #[arg(long)]
        ci: bool,

        /// Run only specific test category
        #[arg(long, value_name = "CATEGORY")]
        filter: Option<String>,

        /// Number of test threads (default: 1 for sequential execution)
        #[arg(long, short, default_value = "1")]
        threads: usize,
    },

    /// Run import unit tests
    Unit {
        /// Run specific test by name filter
        #[arg(long)]
        filter: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Run cache invalidation E2E tests
    E2e {
        /// Run tests in CI mode
        #[arg(long)]
        ci: bool,

        /// Test specific cache type (artwork, scan, metadata)
        #[arg(long)]
        cache_type: Option<String>,
    },

    /// Run cache integration tests
    Integration {
        /// Run specific test by name filter
        #[arg(long)]
        filter: Option<String>,
    },
}

// ============================================================================
// Setup Commands
// ============================================================================

#[derive(Subcommand)]
pub enum SetupCommands {
    /// Install system dependencies (platform-aware)
    Deps {
        /// Print commands instead of running them
        #[arg(long)]
        manual: bool,
    },

    /// Setup SQLx (database, migrations, offline mode)
    Sqlx {
        /// Skip database creation
        #[arg(long)]
        skip_create: bool,

        /// Skip migrations
        #[arg(long)]
        skip_migrate: bool,
    },

    /// Setup environment files (.env from .env.example)
    Env {
        /// Overwrite existing .env file
        #[arg(long)]
        force: bool,
    },

    /// Setup git hooks
    Hooks,

    /// Run complete first-time setup
    All {
        /// Skip confirmation prompts
        #[arg(long, short)]
        yes: bool,
    },
}

// ============================================================================
// Clean Commands
// ============================================================================

#[derive(Subcommand)]
pub enum CleanCommands {
    /// Clean development artifacts
    Dev,

    /// Nuclear clean (node_modules, target, caches)
    Full {
        /// Also clean Cargo cache
        #[arg(long)]
        cargo_cache: bool,
    },

    /// Clear caches (SQLx, build, etc.)
    Cache,
}

// ============================================================================
// Version Commands
// ============================================================================

#[derive(Subcommand)]
pub enum VersionCommands {
    /// Bump version across all workspace files
    Bump {
        /// New version (semver format)
        version: String,

        /// Preview changes without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip git commit/tag/push
        #[arg(long)]
        skip_git: bool,

        /// Allow version bump on non-main branch
        #[arg(long)]
        force: bool,
    },

    /// Show current workspace version
    Current,

    /// Validate semantic version format
    Validate {
        /// Version to validate
        version: String,
    },
}

// ============================================================================
// Dev Commands
// ============================================================================

#[derive(Subcommand)]
pub enum DevCommands {
    /// Run desktop dev server
    Desktop {
        /// Show logs only (skip opening app)
        #[arg(long)]
        logs: bool,
    },

    /// Run mobile dev server
    Mobile {
        /// Target platform (ios, android)
        #[arg(long)]
        platform: Option<String>,
    },

    /// Run marketing site dev server
    Marketing,

    /// Run web app dev server
    Web,

    /// Run backend server (Docker Compose)
    Server {
        /// Run in detached mode
        #[arg(long, short)]
        detached: bool,
    },
}

// ============================================================================
// CI Commands
// ============================================================================

#[derive(Subcommand)]
pub enum CiCommands {
    /// Test Docker build
    DockerBuild {
        /// Show image size
        #[arg(long)]
        show_size: bool,
    },

    /// Validate release preparation
    ValidateRelease,

    /// Export CI metrics
    Metrics {
        /// Output file for metrics
        #[arg(long, short, default_value = "metrics.json")]
        output: String,
    },
}
