use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub struct CrateResolver {
    directory: PathBuf,
}

pub struct ResolvedCrate {
    pub name: String,
    pub version: String,
}

pub enum ResolveError {
    CargoMissing(io::Error),
    Unwritable(io::Error),
    Cargo(String),
}

impl CrateResolver {
    const CARGO: &str = "cargo";
    const MANIFEST: &str = "Cargo.toml";
    const SCRATCH_MANIFEST: &str =
        "[package]\nname = \"scratch\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n";
    const SOURCE: &str = "src/main.rs";
    const VERSION_KEY: &str = "version = \"";
    const DEPENDENCIES_SECTION: &str = "[dependencies]";
    const QUOTE: char = '"';
    const ASSIGN: char = '=';

    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub fn resolve(&self, name: &str, features: &[String]) -> Result<ResolvedCrate, ResolveError> {
        self.write_scratch().map_err(ResolveError::Unwritable)?;
        let output = self
            .add_command(name, features)
            .output()
            .map_err(ResolveError::CargoMissing)?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr)
                .trim_end()
                .to_string();
            return Err(ResolveError::Cargo(message));
        }
        let manifest = fs::read_to_string(self.directory.join(Self::MANIFEST))
            .map_err(ResolveError::Unwritable)?;
        Self::written_crate(&manifest).ok_or_else(|| {
            ResolveError::Cargo(format!("error: cargo added `{name}` without a version"))
        })
    }

    fn write_scratch(&self) -> io::Result<()> {
        let source = self.directory.join(Self::SOURCE);
        fs::create_dir_all(source.parent().unwrap_or(&self.directory))?;
        fs::write(&source, "")?;
        fs::write(self.directory.join(Self::MANIFEST), Self::SCRATCH_MANIFEST)
    }

    fn add_command(&self, name: &str, features: &[String]) -> Command {
        let mut command = Command::new(Self::CARGO);
        command
            .args(["add", "--quiet"])
            .current_dir(&self.directory);
        if !features.is_empty() {
            command.arg(format!("--features={}", features.join(",")));
        }
        command.args(["--", name]);
        command
    }

    fn written_crate(manifest: &str) -> Option<ResolvedCrate> {
        let (_, after_section) = manifest.split_once(Self::DEPENDENCIES_SECTION)?;
        let line = after_section.lines().find(|line| !line.trim().is_empty())?;
        let (key, value) = line.split_once(Self::ASSIGN)?;
        let value = value.trim();
        let quoted = match value.find(Self::VERSION_KEY) {
            Some(start) => &value[start + Self::VERSION_KEY.len()..],
            None => value.strip_prefix(Self::QUOTE)?,
        };
        let version = quoted
            .split(Self::QUOTE)
            .next()
            .filter(|version| !version.is_empty())?;
        Some(ResolvedCrate {
            name: key.trim().trim_matches(Self::QUOTE).to_string(),
            version: version.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CrateResolver;

    fn written(manifest: &str) -> Option<(String, String)> {
        CrateResolver::written_crate(manifest).map(|resolved| (resolved.name, resolved.version))
    }

    #[test]
    fn the_crate_is_read_from_a_plain_or_a_table_entry() {
        let plain = "[package]\nname = \"scratch\"\n\n[dependencies]\nfake-json = \"1.4.2\"\n";
        assert_eq!(
            written(plain),
            Some(("fake-json".to_string(), "1.4.2".to_string()))
        );
        let table =
            "[dependencies]\nfake-json = { version = \"1.4.2\", features = [\"derive\"] }\n";
        assert_eq!(
            written(table),
            Some(("fake-json".to_string(), "1.4.2".to_string()))
        );
    }

    #[test]
    fn the_crate_keeps_the_name_cargo_wrote() {
        let manifest = "[dependencies]\nfake_json = \"1.0\"\n";
        assert_eq!(
            written(manifest),
            Some(("fake_json".to_string(), "1.0".to_string()))
        );
    }

    #[test]
    fn a_manifest_without_a_versioned_dependency_gives_no_crate() {
        assert_eq!(written("[package]\nname = \"scratch\"\n"), None);
        assert_eq!(
            written("[dependencies]\nfake-json = { path = \"../fake\" }\n"),
            None
        );
    }
}
