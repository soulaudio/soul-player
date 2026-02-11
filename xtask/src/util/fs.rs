use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Remove a directory and all its contents (handles Windows file locks gracefully)
pub fn remove_dir_all(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if cfg!(target_os = "windows") {
        // On Windows, retry a few times to handle file locks
        for attempt in 0..3 {
            match fs::remove_dir_all(path) {
                Ok(_) => return Ok(()),
                Err(e) if attempt < 2 => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    return Err(e)
                        .with_context(|| format!("Failed to remove directory: {:?}", path))
                }
            }
        }
    } else {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory: {:?}", path))?;
    }

    Ok(())
}

/// Remove a file (handles Windows file locks gracefully)
pub fn remove_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if cfg!(target_os = "windows") {
        // On Windows, retry a few times to handle file locks
        for attempt in 0..3 {
            match fs::remove_file(path) {
                Ok(_) => return Ok(()),
                Err(e) if attempt < 2 => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    return Err(e).with_context(|| format!("Failed to remove file: {:?}", path))
                }
            }
        }
    } else {
        fs::remove_file(path).with_context(|| format!("Failed to remove file: {:?}", path))?;
    }

    Ok(())
}

/// Find all files matching a pattern
pub fn find_files(root: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.contains(pattern) {
                files.push(path.to_path_buf());
            }
        }
    }

    Ok(files)
}

/// Find all directories matching a name
pub fn find_dirs(root: &Path, name: &str) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        let path = entry.path();
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if dir_name == name {
                dirs.push(path.to_path_buf());
            }
        }
    }

    Ok(dirs)
}

/// Copy a file, creating parent directories if needed
pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::copy(from, to)
        .with_context(|| format!("Failed to copy {} to {}", from.display(), to.display()))?;

    Ok(())
}

/// Write content to a file, creating parent directories if needed
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {:?}", parent))?;
    }

    fs::write(path, content).with_context(|| format!("Failed to write file: {:?}", path))?;

    Ok(())
}

/// Read file content
pub fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))
}

/// Check if a directory is empty
pub fn is_dir_empty(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }

    let mut entries =
        fs::read_dir(path).with_context(|| format!("Failed to read directory: {:?}", path))?;

    Ok(entries.next().is_none())
}
