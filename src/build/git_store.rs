use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use super::user_cache::UserCache;

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
    const DIRECTORY: &str = "git";
    const GIT: &str = "git";
    const NO_PROMPT: (&str, &str) = ("GIT_TERMINAL_PROMPT", "0");
    const ALLOWED_SCHEMES: (&str, &str) = ("GIT_ALLOW_PROTOCOL", "file:git:http:https:ssh");
    const SSH_COMMAND: &str = "GIT_SSH_COMMAND";
    const BATCH_SSH: &str = "ssh -o BatchMode=yes";
    const MESSAGE_STARTS: [&str; 2] = ["fatal:", "error:"];

    pub fn for_user() -> Result<Self, StoreError> {
        let cache = UserCache::directory().ok_or(StoreError::NoCacheDirectory)?;
        Ok(Self {
            root: cache.join(Self::DIRECTORY),
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
        fs::remove_dir_all(&staging).ok();
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
                .args(["clone", "--quiet", "--no-checkout", "--", url])
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
        command
            .env(Self::NO_PROMPT.0, Self::NO_PROMPT.1)
            .env(Self::ALLOWED_SCHEMES.0, Self::ALLOWED_SCHEMES.1);
        if env::var_os(Self::SSH_COMMAND).is_none() {
            command.env(Self::SSH_COMMAND, Self::BATCH_SSH);
        }
        let output = command.output().map_err(StoreError::GitMissing)?;
        if output.status.success() {
            return Ok(());
        }
        Err(StoreError::Fetch(Self::first_message(
            &String::from_utf8_lossy(&output.stderr),
        )))
    }

    fn first_message(stderr: &str) -> String {
        let lines = stderr
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty());
        let first_error = lines.clone().find(|line| {
            Self::MESSAGE_STARTS
                .iter()
                .any(|start| line.starts_with(start))
        });
        first_error
            .or_else(|| lines.clone().next())
            .unwrap_or_default()
            .to_string()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use super::{GitStore, StoreError};

    struct StoreDirectory {
        root: PathBuf,
    }

    impl StoreDirectory {
        const READ_AND_ENTER: u32 = 0o555;
        const WRITABLE: u32 = 0o755;

        fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("git-store-{}-{name}", std::process::id()));
            fs::remove_dir_all(&root).ok();
            fs::create_dir_all(&root).expect("the temporary directory is writable");
            Self { root }
        }

        fn committed_repository(&self) -> String {
            let repository = self.root.join("repository");
            fs::create_dir_all(&repository).expect("the repository directory is writable");
            fs::write(repository.join("kept.txt"), "kept")
                .expect("the repository file is writable");
            for arguments in [
                &["init", "--quiet"][..],
                &["add", "--all"],
                &["commit", "--quiet", "--message", "Add a file"],
            ] {
                Self::git(&repository, arguments);
            }
            Self::git(&repository, &["rev-parse", "HEAD"])
        }

        fn git(directory: &std::path::Path, arguments: &[&str]) -> String {
            let output = std::process::Command::new("git")
                .args(arguments)
                .current_dir(directory)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "Fixture Author")
                .env("GIT_AUTHOR_EMAIL", "author@example.com")
                .env("GIT_COMMITTER_NAME", "Fixture Author")
                .env("GIT_COMMITTER_EMAIL", "author@example.com")
                .output()
                .expect("git starts");
            assert!(output.status.success(), "git {arguments:?} fails");
            String::from_utf8(output.stdout)
                .expect("git prints text")
                .trim()
                .to_string()
        }

        fn staging(&self, name: &str) -> PathBuf {
            let staging = self.root.join(name);
            fs::create_dir_all(staging.join("src")).expect("the staging directory is writable");
            staging
        }
    }

    impl Drop for StoreDirectory {
        fn drop(&mut self) {
            fs::set_permissions(&self.root, fs::Permissions::from_mode(Self::WRITABLE)).ok();
            fs::remove_dir_all(&self.root).ok();
        }
    }

    #[test]
    fn a_second_settle_of_one_commit_keeps_the_first_and_removes_its_staging() {
        let store = StoreDirectory::new("race");
        let destination = store.root.join("commit");
        let first = store.staging("commit.1");
        let second = store.staging("commit.2");
        assert!(GitStore::settle(&first, destination.clone()).is_ok());
        assert!(GitStore::settle(&second, destination.clone()).is_ok());
        assert!(destination.is_dir());
        assert!(!second.exists());
    }

    #[test]
    fn a_settle_into_an_unwritable_store_is_unwritable() {
        let store = StoreDirectory::new("unwritable");
        let staging = store.staging("commit.1");
        let destination = store.root.join("commit");
        fs::set_permissions(
            &store.root,
            fs::Permissions::from_mode(StoreDirectory::READ_AND_ENTER),
        )
        .expect("the temporary directory takes permissions");
        let settled = GitStore::settle(&staging, destination);
        assert!(matches!(settled, Err(StoreError::Unwritable(..))));
    }

    #[test]
    fn a_checkout_clears_a_leftover_staging_directory() {
        let store = StoreDirectory::new("leftover");
        let commit = store.committed_repository();
        let leftover = store
            .root
            .join("store")
            .join(format!("{commit}.{}", std::process::id()));
        fs::create_dir_all(&leftover).expect("the leftover directory is writable");
        fs::write(leftover.join("left.txt"), "left").expect("the leftover file is writable");
        let git_store = GitStore {
            root: store.root.join("store"),
        };
        let repository = store.root.join("repository");
        let checkout = git_store
            .checkout(&repository.display().to_string(), &commit)
            .unwrap_or_else(|_| panic!("the checkout succeeds"));
        assert!(checkout.join("kept.txt").is_file());
        assert!(!checkout.join("left.txt").exists());
    }

    #[test]
    fn the_message_is_the_first_fatal_or_error_line() {
        let stderr = "ssh: Could not resolve hostname\nfatal: Could not read from remote repository.\n\nPlease make sure\nand the repository exists.\n";
        assert_eq!(
            GitStore::first_message(stderr),
            "fatal: Could not read from remote repository."
        );
        assert_eq!(
            GitStore::first_message("\nsomething went wrong\nmore\n"),
            "something went wrong"
        );
    }
}
