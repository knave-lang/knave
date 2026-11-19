use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct TemporaryRepository {
    root: PathBuf,
}

impl TemporaryRepository {
    const UNSET_GLOBAL_CONFIG: (&str, &str) = ("GIT_CONFIG_GLOBAL", "/dev/null");
    const UNSET_SYSTEM_CONFIG: (&str, &str) = ("GIT_CONFIG_NOSYSTEM", "1");
    const AUTHOR: [&str; 4] = [
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
    ];

    pub fn outside_git(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("xtask-{name}-{}", std::process::id()));
        fs::create_dir_all(&root).expect("the directory is created");
        Self { root }
    }

    pub fn initialized(name: &str) -> Self {
        let repository = Self::outside_git(name);
        repository.git(&["init", "--quiet"]);
        repository
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, file: &str, text: &str) {
        let path = self.root.join(file);
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory).expect("the directory is created");
        }
        fs::write(path, text).expect("the file is written");
    }

    pub fn remove(&self, file: &str) {
        fs::remove_file(self.root.join(file)).expect("the file is removed");
    }

    pub fn commit_all(&self) {
        self.commit_all_with_message("fixture");
    }

    pub fn commit_all_with_message(&self, message: &str) {
        self.git(&["add", "."]);
        self.git(&[&Self::AUTHOR[..], &["commit", "--quiet", "-m", message]].concat());
    }

    pub fn git(&self, arguments: &[&str]) {
        let status = Command::new("git")
            .args(arguments)
            .env(Self::UNSET_GLOBAL_CONFIG.0, Self::UNSET_GLOBAL_CONFIG.1)
            .env(Self::UNSET_SYSTEM_CONFIG.0, Self::UNSET_SYSTEM_CONFIG.1)
            .current_dir(&self.root)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {arguments:?} failed");
    }
}

impl Drop for TemporaryRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
