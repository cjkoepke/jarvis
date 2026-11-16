use crate::files::copy_dir;
use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

static PACKAGED: Dir = include_dir!("$CARGO_MANIFEST_DIR/standards");

/// Where an install takes its standards from, before a person's own are laid over them.
pub enum Source {
    /// The standards built into this binary.
    Packaged,
    Folder(PathBuf),
}

impl Source {
    pub fn copy_into(&self, dir: &Path) -> Result<()> {
        match self {
            Source::Packaged => {
                fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
                PACKAGED
                    .extract(dir)
                    .with_context(|| format!("writing the packaged standards to {}", dir.display()))
            }
            Source::Folder(folder) => copy_dir(folder, dir),
        }
    }

    pub fn folder(&self) -> Option<&Path> {
        match self {
            Source::Packaged => None,
            Source::Folder(folder) => Some(folder),
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Source::Packaged => write!(formatter, "the standards packaged with jarvis"),
            Source::Folder(folder) => write!(formatter, "{}", folder.display()),
        }
    }
}
