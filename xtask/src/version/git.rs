use anyhow::Result;
use std::path::PathBuf;

use crate::util::{git, output, platform};

/// Check if git working tree is clean
pub fn check_clean_working_tree() -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    if !git::is_working_tree_clean(&repo)? {
        output::print_error("Git working directory is not clean");
        println!();

        // Show modified files
        let modified = git::modified_files(&repo)?;
        println!("Uncommitted changes:");
        for file in &modified {
            println!("  {}", file);
        }
        println!();

        output::print_info("Please commit or stash your changes before bumping version");
        anyhow::bail!("Git working tree is not clean");
    }

    output::print_success("Git working directory is clean");

    Ok(())
}

/// Check if on main or master branch
pub fn check_on_main_branch(force: bool) -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    let branch = git::current_branch(&repo)?;

    if branch != "main" && branch != "master" {
        if force {
            output::print_warning(&format!(
                "On branch '{}' (not main/master) - forced",
                branch
            ));
        } else {
            output::print_error(&format!(
                "You are on branch '{}', not 'main' or 'master'",
                branch
            ));
            output::print_info("Version bumps are typically done on the main branch");
            output::print_info("Use --force to bypass this check");
            anyhow::bail!("Not on main branch");
        }
    } else {
        output::print_success(&format!("On branch '{}'", branch));
    }

    Ok(())
}

/// Stage all modified files
pub fn stage_files(files: Vec<PathBuf>) -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    // Stage all files
    git::stage_all(&repo)?;

    output::print_success(&format!("Staged {} files", files.len()));

    Ok(())
}

/// Create version bump commit
pub fn create_commit(version: &str) -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    let message = format!(
        "chore(release): bump version to {}\n\nCo-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>",
        version
    );

    git::commit(&repo, &message)?;

    output::print_success(&format!(
        "Commit: chore(release): bump version to {}",
        version
    ));

    Ok(())
}

/// Create git tag
pub fn create_tag(version: &str) -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    let tag_name = format!("v{}", version);
    let message = format!("Release {}", version);

    git::create_tag(&repo, &tag_name, &message)?;

    output::print_success(&format!("Tag: {}", tag_name));

    Ok(())
}

/// Push to origin (main branch)
pub fn push_to_origin() -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    let branch = git::current_branch(&repo)?;
    let refspec = format!("refs/heads/{}", branch);

    git::push(&repo, &refspec)?;

    output::print_success(&format!("Pushed to origin/{}", branch));

    Ok(())
}

/// Push tag to origin
pub fn push_tag(version: &str) -> Result<()> {
    let workspace_root = platform::workspace_root()?;
    let repo = git::get_repo(&workspace_root)?;

    let tag_name = format!("v{}", version);
    let refspec = format!("refs/tags/{}", tag_name);

    git::push(&repo, &refspec)?;

    output::print_success(&format!("Pushed tag {}", tag_name));

    Ok(())
}

/// Run all git operations for version bump
pub fn run_git_operations(version: &str, files: Vec<PathBuf>) -> Result<()> {
    output::print_header("Git Operations");

    stage_files(files)?;
    create_commit(version)?;
    create_tag(version)?;
    push_to_origin()?;
    push_tag(version)?;

    Ok(())
}
