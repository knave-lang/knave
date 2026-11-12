use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct ScratchDirectory {
    path: PathBuf,
}

impl ScratchDirectory {
    const PREFIX: &str = "knave-agree";

    pub fn create() -> io::Result<Self> {
        static CREATED: AtomicU64 = AtomicU64::new(0);
        let number = CREATED.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("{}-{}-{number}", Self::PREFIX, process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
