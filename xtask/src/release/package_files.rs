use super::release_version::ReleaseVersion;
use super::toml_text::TomlText;
use crate::failure::TaskFailure;

pub struct PackageFiles {
    manifest: TomlText,
    lock: TomlText,
}

impl PackageFiles {
    const PACKAGE_TABLE: &'static str = "[package]";
    const WORKSPACE_PACKAGE_TABLE: &'static str = "[workspace.package]";
    const WORKSPACE_DEPENDENCY_TABLE_PREFIX: &'static str = "[workspace.dependencies.";
    const TABLE_END: &'static str = "]";
    const EXACT_REQUIREMENT: &'static str = "=";
    const NAME_KEY: &'static str = "name";
    const VERSION_KEY: &'static str = "version";
    const MANIFEST_NAME: &'static str = "the manifest";
    const LOCK_NAME: &'static str = "the lock file";

    pub fn new(manifest: TomlText, lock: TomlText) -> Self {
        Self { manifest, lock }
    }

    pub fn manifest(&self) -> &str {
        self.manifest.as_str()
    }

    pub fn lock(&self) -> &str {
        self.lock.as_str()
    }

    pub fn versioned(&self, version: &ReleaseVersion) -> Result<Self, TaskFailure> {
        let version = version.to_string();
        Ok(Self::new(
            self.versioned_manifest(&version)?,
            self.versioned_lock(&version)?,
        ))
    }

    fn versioned_manifest(&self, version: &str) -> Result<TomlText, TaskFailure> {
        let workspace = Self::with_version(
            &self.manifest,
            Self::MANIFEST_NAME,
            Self::WORKSPACE_PACKAGE_TABLE,
            version,
        )?;
        let requirement = format!("{}{version}", Self::EXACT_REQUIREMENT);
        self.pinned_crates().try_fold(workspace, |manifest, name| {
            let table = format!(
                "{}{name}{}",
                Self::WORKSPACE_DEPENDENCY_TABLE_PREFIX,
                Self::TABLE_END
            );
            Self::with_version(&manifest, Self::MANIFEST_NAME, &table, &requirement)
        })
    }

    fn versioned_lock(&self, version: &str) -> Result<TomlText, TaskFailure> {
        let package = self.package_name()?;
        std::iter::once(package)
            .chain(self.pinned_crates())
            .try_fold(self.lock.clone(), |lock, name| {
                let entry = format!("{} = \"{name}\"", Self::NAME_KEY);
                Self::with_version(&lock, Self::LOCK_NAME, &entry, version)
            })
    }

    fn pinned_crates(&self) -> impl Iterator<Item = &str> {
        self.manifest
            .table_names(Self::WORKSPACE_DEPENDENCY_TABLE_PREFIX, Self::TABLE_END)
    }

    fn package_name(&self) -> Result<&str, TaskFailure> {
        self.manifest
            .string_value(Self::PACKAGE_TABLE, Self::NAME_KEY)
            .ok_or_else(|| {
                TaskFailure::manifest_key_missing(
                    Self::MANIFEST_NAME,
                    Self::PACKAGE_TABLE,
                    Self::NAME_KEY,
                )
            })
    }

    fn with_version(
        text: &TomlText,
        file: &str,
        anchor: &str,
        version: &str,
    ) -> Result<TomlText, TaskFailure> {
        text.with_string_value(anchor, Self::VERSION_KEY, version)
            .ok_or_else(|| TaskFailure::manifest_key_missing(file, anchor, Self::VERSION_KEY))
    }
}

#[cfg(test)]
mod tests {
    use super::PackageFiles;
    use crate::release::release_version::ReleaseVersion;
    use crate::release::toml_text::TomlText;

    struct PackageFilesFixture;

    impl PackageFilesFixture {
        const MANIFEST: &'static str = include_str!("../../fixtures/release/manifest.toml");
        const LOCK: &'static str = include_str!("../../fixtures/release/lock.toml");
        const VERSIONED_MANIFEST: &'static str =
            include_str!("../../fixtures/release/versioned-manifest.toml");
        const VERSIONED_LOCK: &'static str =
            include_str!("../../fixtures/release/versioned-lock.toml");

        fn versioned(version: &str) -> PackageFiles {
            let files = PackageFiles::new(
                TomlText::new(Self::MANIFEST.to_string()),
                TomlText::new(Self::LOCK.to_string()),
            );
            files
                .versioned(&ReleaseVersion::parse(version).unwrap())
                .unwrap()
        }
    }

    #[test]
    fn the_manifest_changes_the_workspace_version_and_each_pin() {
        assert_eq!(
            PackageFilesFixture::versioned("0.1.1").manifest(),
            PackageFilesFixture::VERSIONED_MANIFEST
        );
    }

    #[test]
    fn the_lock_file_changes_only_the_released_entries() {
        assert_eq!(
            PackageFilesFixture::versioned("0.1.1").lock(),
            PackageFilesFixture::VERSIONED_LOCK
        );
    }
}
