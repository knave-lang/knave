use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::build_directory::BuildDirectory;
use crate::build::{CargoManifest, CargoTarget, CargoTargetKind, DirectoryHash};
use crate::project::CrateDependency;

#[derive(Clone)]
pub struct ProjectCrates {
    root: PathBuf,
    lock_file: PathBuf,
    name: String,
    crates: Vec<CrateDependency>,
    dev_crates: Vec<CrateDependency>,
}

impl ProjectCrates {
    const TEST_CRATE_DIRECTORY: &str = "test_runner";
    const LOCK_FILE: &str = "Cargo.lock";

    pub fn new(
        root: PathBuf,
        name: String,
        crates: Vec<CrateDependency>,
        dev_crates: Vec<CrateDependency>,
    ) -> Self {
        Self {
            lock_file: root.join(Self::LOCK_FILE),
            root,
            name,
            crates,
            dev_crates,
        }
    }

    pub fn locked_at(&self, directory: &Path) -> Self {
        Self {
            lock_file: directory.join(Self::LOCK_FILE),
            ..self.clone()
        }
    }

    pub fn copy_lock_into(&self, copy: &ProjectCrates) -> io::Result<()> {
        if !self.lock_file.is_file() {
            return Ok(());
        }
        if let Some(folder) = copy.lock_file.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::copy(&self.lock_file, &copy.lock_file).map(|_| ())
    }

    pub fn has_crates(&self) -> bool {
        !self.crates.is_empty()
    }

    pub fn program_target(
        &self,
        name_stem: &str,
        directory: PathBuf,
        output: PathBuf,
    ) -> Option<CargoTarget<'_>> {
        self.has_crates().then(|| {
            let manifest = self.manifest(CargoTargetKind::Bin, name_stem, &directory);
            self.target(manifest, directory, output)
        })
    }

    pub fn test_target(&self, output: PathBuf) -> Option<CargoTarget<'_>> {
        let directory = self.build_directory().join(Self::TEST_CRATE_DIRECTORY);
        let tests_use_crates = self.has_crates() || !self.dev_crates.is_empty();
        tests_use_crates.then(|| {
            let manifest = self.manifest(CargoTargetKind::Test, &self.name, &directory);
            self.target(manifest, directory, output)
        })
    }

    pub fn build_directory(&self) -> PathBuf {
        self.root.join(BuildDirectory::root()).join(&self.name)
    }

    fn manifest(
        &self,
        kind: CargoTargetKind,
        name_stem: &str,
        directory: &Path,
    ) -> CargoManifest<'_> {
        CargoManifest::new(
            kind,
            kind.target_name(name_stem, &DirectoryHash::of(directory)),
            &self.crates,
            &self.dev_crates,
        )
    }

    fn target<'c>(
        &'c self,
        manifest: CargoManifest<'c>,
        directory: PathBuf,
        output: PathBuf,
    ) -> CargoTarget<'c> {
        CargoTarget {
            manifest,
            directory,
            lock_file: self.lock_file.clone(),
            output,
        }
    }
}
