// ! Soul Player xtask - Development Task Automation
//!
//! This crate provides development automation tools for Soul Player.
//! Run `cargo xtask --help` to see all available commands.

use anyhow::Result;
use clap::Parser;

mod cli;
mod util;

// Command modules
mod build;
mod check;
mod ci;
mod clean;
mod dev;
mod setup;
mod test;
mod version;

use cli::{
    AudioCommands, BuildCommands, CacheCommands, CheckCommands, CiCommands, CleanCommands, Cli,
    Commands, DevCommands, ImportCommands, SetupCommands, TestCommands, VersionCommands,
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // ================================================================
        // Check Commands
        // ================================================================
        Commands::Check(cmd) => match cmd {
            CheckCommands::Precommit => check::precommit::run(),
            CheckCommands::Ci => check::ci::run(),
            CheckCommands::Fmt { fix } => check::rust::run_fmt(fix),
            CheckCommands::Clippy { fix } => check::rust::run_clippy(fix),
            CheckCommands::Test { package, args } => check::rust::run_tests(package, args),
            CheckCommands::Typescript { workspace } => check::typescript::run_typescript(workspace),
            CheckCommands::Lint { fix, workspace } => check::typescript::run_lint(fix, workspace),
        },

        // ================================================================
        // Build Commands
        // ================================================================
        Commands::Build(cmd) => match cmd {
            BuildCommands::Desktop { release } => build::desktop::run(release),
            BuildCommands::Mobile { release, platform } => build::mobile::run(release, platform),
            BuildCommands::Wasm { watch, release } => build::wasm::run(watch, release),
            BuildCommands::Marketing { release } => build::marketing::run(release),
            BuildCommands::Web { release } => build::web::run(release),
            BuildCommands::All { release } => build::desktop::run_all(release),
        },

        // ================================================================
        // Test Commands
        // ================================================================
        Commands::Test(cmd) => match cmd {
            TestCommands::Audio(audio_cmd) => match audio_cmd {
                AudioCommands::E2e {
                    ci,
                    skip_device_check,
                    init_only,
                    stutter_only,
                    export_metrics,
                } => test::audio::run_e2e_tests(
                    ci,
                    skip_device_check,
                    init_only,
                    stutter_only,
                    export_metrics,
                ),
                AudioCommands::ListDevices { verbose, filter } => {
                    test::devices::list_devices(verbose, filter.as_deref())
                }
                AudioCommands::GenerateAssets { output, force } => {
                    test::audio::generate_assets(&output, force)
                }
                AudioCommands::Ci {
                    timeout,
                    export_metrics,
                } => test::audio::run_ci_tests(timeout, export_metrics),
            },

            TestCommands::Import(import_cmd) => match import_cmd {
                ImportCommands::E2e {
                    ci,
                    filter,
                    threads,
                } => test::import::run_e2e_tests(ci, filter, threads),
                ImportCommands::Unit { filter } => test::import::run_unit_tests(filter),
            },

            TestCommands::Cache(cache_cmd) => match cache_cmd {
                CacheCommands::E2e { ci, cache_type } => test::cache::run_e2e_tests(ci, cache_type),
                CacheCommands::Integration { filter } => test::cache::run_integration_tests(filter),
            },

            TestCommands::Unit { package, args } => test::e2e::run_unit_tests(package, args),
            TestCommands::Integration => test::e2e::run_integration_tests(),
            TestCommands::E2e { suite } => test::e2e::run_e2e_tests(suite),
        },

        // ================================================================
        // Setup Commands
        // ================================================================
        Commands::Setup(cmd) => match cmd {
            SetupCommands::Deps { manual } => setup::deps::run(manual),
            SetupCommands::Sqlx {
                skip_create,
                skip_migrate,
            } => setup::sqlx::run(skip_create, skip_migrate),
            SetupCommands::Env { force } => setup::env::run(force),
            SetupCommands::Hooks => setup::hooks::run(),
            SetupCommands::All { yes } => setup::deps::run_all(yes),
        },

        // ================================================================
        // Clean Commands
        // ================================================================
        Commands::Clean(cmd) => match cmd {
            CleanCommands::Dev => clean::dev::run(),
            CleanCommands::Full { cargo_cache } => clean::full::run(cargo_cache),
            CleanCommands::Cache => clean::cache::run(),
        },

        // ================================================================
        // Version Commands
        // ================================================================
        Commands::Version(cmd) => match cmd {
            VersionCommands::Bump {
                version,
                dry_run,
                skip_git,
                force,
            } => version::bump::run(&version, dry_run, skip_git, force),
            VersionCommands::Current => version::validate::show_current(),
            VersionCommands::Validate { version } => version::validate::validate(&version),
        },

        // ================================================================
        // Dev Commands
        // ================================================================
        Commands::Dev(cmd) => match cmd {
            DevCommands::Desktop { logs } => dev::desktop::run(logs),
            DevCommands::Mobile { platform } => dev::mobile::run(platform),
            DevCommands::Marketing => dev::marketing::run(),
            DevCommands::Web => dev::web::run(),
            DevCommands::Server { detached } => dev::server::run(detached),
        },

        // ================================================================
        // CI Commands
        // ================================================================
        Commands::Ci(cmd) => match cmd {
            CiCommands::DockerBuild { show_size } => ci::docker::run(show_size),
            CiCommands::ValidateRelease => ci::release::run(),
            CiCommands::Metrics { output } => ci::metrics::run(&output),
        },
    }
}
