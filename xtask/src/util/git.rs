use anyhow::{Context, Result};
use git2::{Repository, StatusOptions};
use std::path::Path;

/// Get the git repository for the workspace
pub fn get_repo(workspace_root: &Path) -> Result<Repository> {
    Repository::open(workspace_root).context("Failed to open git repository")
}

/// Check if the git working tree is clean (no uncommitted changes)
pub fn is_working_tree_clean(repo: &Repository) -> Result<bool> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses.is_empty())
}

/// Get the current branch name
pub fn current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let branch = head
        .shorthand()
        .ok_or_else(|| anyhow::anyhow!("Failed to get branch name"))?;
    Ok(branch.to_string())
}

/// Check if the current branch is main or master
pub fn is_main_branch(repo: &Repository) -> Result<bool> {
    let branch = current_branch(repo)?;
    Ok(branch == "main" || branch == "master")
}

/// Stage all modified files
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

/// Create a commit with a message
pub fn commit(repo: &Repository, message: &str) -> Result<git2::Oid> {
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let signature = repo.signature()?;
    let parent_commit = repo.head()?.peel_to_commit()?;

    let oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;

    Ok(oid)
}

/// Create a tag
pub fn create_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<()> {
    let obj = repo.revparse_single("HEAD")?;
    let signature = repo.signature()?;

    repo.tag(tag_name, &obj, &signature, message, false)?;

    Ok(())
}

/// Push to remote
pub fn push(repo: &Repository, refspec: &str) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;

    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(callbacks);

    remote.push(&[refspec], Some(&mut push_options))?;

    Ok(())
}

/// Get list of modified files
pub fn modified_files(repo: &Repository) -> Result<Vec<String>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(false);
    opts.include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut files = Vec::new();

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            files.push(path.to_string());
        }
    }

    Ok(files)
}
