use crate::files::find_on_path;
use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TRACKING_FILE: &str = ".crates.toml";

/// A binary `cargo install` put in place, as cargo's own record of installs describes it.
pub struct Tracked {
    root: PathBuf,
    package: String,
    source: Source,
}

#[derive(Debug, PartialEq)]
enum Source {
    Checkout(PathBuf),
    Git { url: String, branch: Option<String> },
    PinnedGit { url: String, pinned_to: String },
    Registry,
}

impl Tracked {
    /// Cargo keeps a binary at `<root>/bin/` and lists it in `<root>/.crates.toml`.
    pub fn at(real_executable: &Path) -> Option<Tracked> {
        let bin = real_executable.parent().filter(|bin| bin.ends_with("bin"))?;
        let root = bin.parent()?;
        let binary = real_executable.file_name()?.to_str()?;

        let tracking: toml::Table = fs::read_to_string(root.join(TRACKING_FILE)).ok()?.parse().ok()?;
        let installs = tracking.get("v1")?.as_table()?;
        let lists_the_binary = |binaries: &toml::Value| {
            binaries
                .as_array()
                .is_some_and(|all| all.iter().any(|one| one.as_str() == Some(binary)))
        };
        let (install, _) = installs.iter().find(|(_, binaries)| lists_the_binary(binaries))?;
        let (package, source) = package_and_source(install)?;
        Some(Tracked {
            root: root.to_path_buf(),
            package,
            source,
        })
    }

    /// Cargo's way to upgrade is to install again from the same source.
    pub fn upgrade(&self, home: &Path) -> Result<Command> {
        let source_arguments = match &self.source {
            Source::Checkout(checkout) if !checkout.join("Cargo.toml").is_file() => {
                bail!(
                    "cargo built this jarvis from {}, which is gone; install it again",
                    checkout.display()
                )
            }
            Source::Checkout(checkout) => vec!["--path".to_string(), checkout.to_string_lossy().into_owned()],
            Source::Git { url, branch } => {
                let on_its_branch = branch.iter().flat_map(|branch| ["--branch".to_string(), branch.clone()]);
                ["--git".to_string(), url.clone()]
                    .into_iter()
                    .chain(on_its_branch)
                    .chain([self.package.clone()])
                    .collect()
            }
            Source::PinnedGit { url, pinned_to } => {
                bail!("cargo installed this jarvis from {url} at {pinned_to}, which never moves; install the one you want with `cargo install --git {url}`")
            }
            Source::Registry => vec![self.package.clone()],
        };
        let Some(cargo) = self.cargo() else {
            bail!(
                "cargo installed this jarvis, but `cargo` is not on PATH; run `cargo install --locked --root {} {}` where it is",
                self.root.display(),
                source_arguments.join(" ")
            );
        };
        let mut install_again = Command::new(cargo);
        install_again
            .args(["install", "--locked", "--root"])
            .arg(&self.root)
            .args(source_arguments);
        install_again.current_dir(home);
        Ok(install_again)
    }

    pub fn outcome(&self) -> String {
        match &self.source {
            Source::Checkout(checkout) => format!(
                "cargo rebuilt jarvis from {} as it is; pull there first for newer commits",
                checkout.display()
            ),
            _ => "cargo has the newest jarvis in place".to_string(),
        }
    }

    fn cargo(&self) -> Option<PathBuf> {
        let beside_the_binary = self.root.join("bin/cargo");
        let in_cargo_home = std::env::var_os("CARGO_HOME").map(|cargo_home| PathBuf::from(cargo_home).join("bin/cargo"));
        let elsewhere = [Some(beside_the_binary), in_cargo_home]
            .into_iter()
            .flatten()
            .find(|cargo| cargo.is_file());
        find_on_path("cargo").or(elsewhere)
    }
}

/// Reads one key of cargo's record, such as `jarvis 0.1.0 (git+https://host/repo?branch=dev#<commit>)`.
fn package_and_source(install: &str) -> Option<(String, Source)> {
    let (package, version_and_source) = install.split_once(' ')?;
    let (_version, source) = version_and_source.split_once(" (")?;
    let (kind, location) = source.strip_suffix(')')?.split_once('+')?;
    let source = match kind {
        "path" => Source::Checkout(PathBuf::from(percent_decoded(location.strip_prefix("file://")?))),
        "git" => {
            let (location, _commit) = location.rsplit_once('#').unwrap_or((location, ""));
            let (url, reference) = location.split_once('?').unwrap_or((location, ""));
            match reference.split_once('=') {
                Some(("branch", branch)) => Source::Git {
                    url: url.to_string(),
                    branch: Some(branch.to_string()),
                },
                Some((_tag_or_rev, pinned_to)) => Source::PinnedGit {
                    url: url.to_string(),
                    pinned_to: pinned_to.to_string(),
                },
                None => Source::Git {
                    url: url.to_string(),
                    branch: None,
                },
            }
        }
        "registry" | "sparse" => Source::Registry,
        _ => return None,
    };
    Some((package.to_string(), source))
}

fn percent_decoded(text: &str) -> String {
    let mut bytes = Vec::new();
    let mut rest = text.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        let escaped = after.get(..2).and_then(|hex| std::str::from_utf8(hex).ok());
        match escaped.filter(|_| byte == b'%').and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
            Some(unescaped) => {
                bytes.push(unescaped);
                rest = &after[2..];
            }
            None => {
                bytes.push(byte);
                rest = after;
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_of_install_cargo_records_is_read_as_the_source_to_install_from_again() {
        let source_of = |install: &str| package_and_source(install).map(|(_, source)| source);
        let from_git = |url: &str, branch: Option<&str>| Source::Git {
            url: url.to_string(),
            branch: branch.map(str::to_string),
        };

        assert_eq!(
            source_of("jarvis 0.0.1 (path+file:///Users/me/my%20repos/jarvis)"),
            Some(Source::Checkout(PathBuf::from("/Users/me/my repos/jarvis")))
        );
        assert_eq!(
            source_of("jarvis 0.0.1 (git+https://host/jarvis#65bbac40)"),
            Some(from_git("https://host/jarvis", None))
        );
        assert_eq!(
            source_of("jarvis 0.0.1 (git+https://host/jarvis?branch=dev#65bbac40)"),
            Some(from_git("https://host/jarvis", Some("dev")))
        );
        assert_eq!(
            source_of("jarvis 0.0.1 (git+https://host/jarvis?tag=v1#65bbac40)"),
            Some(Source::PinnedGit {
                url: "https://host/jarvis".to_string(),
                pinned_to: "v1".to_string()
            })
        );
        assert_eq!(
            source_of("jarvis 0.0.1 (registry+https://github.com/rust-lang/crates.io-index)"),
            Some(Source::Registry)
        );
        assert_eq!(source_of("jarvis 0.0.1 (elsewhere+somewhere)"), None);
    }
}
