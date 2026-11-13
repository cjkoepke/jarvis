use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    for entry in sorted_entries(from)? {
        let destination = to.join(entry.file_name().unwrap_or_default());
        if entry.is_dir() {
            copy_dir(&entry, &destination)?;
        } else {
            fs::copy(&entry, &destination).with_context(|| format!("copying to {}", destination.display()))?;
        }
    }
    Ok(())
}

pub fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        entries.push(entry?.path());
    }
    entries.sort();
    Ok(entries)
}

pub fn has_extension(path: &Path, extension: &str) -> bool {
    path.is_file() && path.extension().is_some_and(|found| found == extension)
}

pub fn find_on_path(executable: &str) -> Option<PathBuf> {
    let search_path = std::env::var_os("PATH")?;
    std::env::split_paths(&search_path)
        .map(|dir| dir.join(executable))
        .find(|candidate| candidate.is_file())
}
