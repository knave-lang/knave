use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use crate::language::Language;

pub struct GitStore {
    root: PathBuf,
}

pub enum StoreError {
    NoCacheDirectory,
    GitMissing(io::Error),
    Unwritable(PathBuf, io::Error),
    Fetch(String),
}

impl GitStore {
    const CACHE_HOME: &str = "XDG_CACHE_HOME";
    const HOME: &str = "HOME";
    const CACHE_UNDER_HOME: &str = ".cache";
    const DIRECTORY: &str = "git";
    const GIT: &str = "git";
    const NO_PROMPT: (&str, &str) = ("GIT_TERMINAL_PROMPT", "0");

    pub fn for_user() -> Result<Self, StoreError> {
        let cache = Self::variable_path(Self::CACHE_HOME)
            .or_else(|| {
                Self::variable_path(Self::HOME).map(|home| home.join(Self::CACHE_UNDER_HOME))
            })
            .ok_or(StoreError::NoCacheDirectory)?;
        Ok(Self {
            root: cache.join(Language::NAME).join(Self::DIRECTORY),
        })
    }

    pub fn checkout(&self, url: &str, commit: &str) -> Result<PathBuf, StoreError> {
        let destination = self.root.join(commit);
        if destination.is_dir() {
            return Ok(destination);
        }
        fs::create_dir_all(&self.root)
            .map_err(|error| StoreError::Unwritable(self.root.clone(), error))?;
        let staging = self.root.join(format!("{commit}.{}", process::id()));
        let fetched = Self::fetch(url, commit, &staging);
        if fetched.is_err() {
            fs::remove_dir_all(&staging).ok();
        }
        fetched?;
        Self::settle(&staging, destination)
    }

    fn fetch(url: &str, commit: &str, staging: &Path) -> Result<(), StoreError> {
        let staging_name = staging.as_os_str();
        Self::run(
            Command::new(Self::GIT)
                .args(["clone", "--quiet", "--no-checkout", url])
                .arg(staging_name),
        )?;
        Self::run(Command::new(Self::GIT).arg("-C").arg(staging_name).args([
            "-c",
            "advice.detachedHead=false",
            "checkout",
            "--quiet",
            "--detach",
            commit,
        ]))
    }

    fn settle(staging: &Path, destination: PathBuf) -> Result<PathBuf, StoreError> {
        match fs::rename(staging, &destination) {
            Ok(()) => Ok(destination),
            Err(_) if destination.is_dir() => {
                fs::remove_dir_all(staging).ok();
                Ok(destination)
            }
            Err(error) => Err(StoreError::Unwritable(destination, error)),
        }
    }

    fn run(command: &mut Command) -> Result<(), StoreError> {
        let output = command
            .env(Self::NO_PROMPT.0, Self::NO_PROMPT.1)
            .output()
            .map_err(StoreError::GitMissing)?;
        if output.status.success() {
            return Ok(());
        }
        let message = String::from_utf8_lossy(&output.stderr);
        Err(StoreError::Fetch(
            message
                .lines()
                .last()
                .unwrap_or_default()
                .trim()
                .to_string(),
        ))
    }

    fn variable_path(name: &str) -> Option<PathBuf> {
        env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
}
