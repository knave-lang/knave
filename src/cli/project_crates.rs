use std::path::PathBuf;

use super::build_directory::BuildDirectory;
use super::program_file::ProgramFile;
use crate::build::{CargoManifest, CargoTarget, CargoTargetKind};
use crate::project::CrateDependency;

pub struct ProjectCrates {
    root: PathBuf,
    name: String,
    crates: Vec<CrateDependency>,
    dev_crates: Vec<CrateDependency>,
}

impl ProjectCrates {
    const TEST_CRATE_DIRECTORY: &str = "test_runner";
    const LOCK_FILE: &str = "Cargo.lock";
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    const SHORT_HASH_MASK: u64 = 0xffff_ffff;

    pub fn new(
        root: PathBuf,
        name: String,
        crates: Vec<CrateDependency>,
        dev_crates: Vec<CrateDependency>,
    ) -> Self {
        Self {
            root,
            name,
            crates,
            dev_crates,
        }
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
            let manifest = self.manifest(CargoTargetKind::Bin, name_stem);
            self.target(manifest, directory, output)
        })
    }

    pub fn test_target(&self, output: PathBuf) -> Option<CargoTarget<'_>> {
        let directory = self.build_directory().join(Self::TEST_CRATE_DIRECTORY);
        let tests_use_crates = self.has_crates() || !self.dev_crates.is_empty();
        tests_use_crates.then(|| {
            let manifest = self.manifest(CargoTargetKind::Test, &self.name);
            self.target(manifest, directory, output)
        })
    }

    pub fn build_directory(&self) -> PathBuf {
        self.root.join(BuildDirectory::root()).join(&self.name)
    }

    fn manifest(&self, kind: CargoTargetKind, name_stem: &str) -> CargoManifest<'_> {
        CargoManifest::new(
            kind,
            kind.target_name(name_stem, &self.root_hash()),
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
            lock_file: self.root.join(Self::LOCK_FILE),
            output,
        }
    }

    fn root_hash(&self) -> String {
        let root = ProgramFile::canonical(&self.root).unwrap_or(self.root.clone());
        let hash = root
            .as_os_str()
            .as_encoded_bytes()
            .iter()
            .fold(Self::FNV_OFFSET, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(Self::FNV_PRIME)
            });
        format!("{:08x}", hash & Self::SHORT_HASH_MASK)
    }
}
