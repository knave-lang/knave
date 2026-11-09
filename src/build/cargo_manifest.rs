use super::build_plan::BuildPlan;
use crate::language::Language;
use crate::project::CrateDependency;

pub struct CargoManifest<'p> {
    pub bin_name: String,
    pub dependencies: &'p [CrateDependency],
    pub dev_dependencies: &'p [CrateDependency],
}

impl CargoManifest<'_> {
    const PACKAGE_SUFFIX: &'static str = "-program";
    const SOURCE_PATH: &'static str = "src/main.rs";

    pub fn package_name() -> String {
        format!("{}{}", Language::NAME, Self::PACKAGE_SUFFIX)
    }

    pub fn render(&self, plan: &BuildPlan) -> String {
        let mut text = format!(
            "[package]\nname = {}\nversion = \"0.1.0\"\nedition = {}\npublish = false\n\
             autobins = false\nautotests = false\nautoexamples = false\nautobenches = false\n\n\
             [[bin]]\nname = {}\npath = {}\n\n[workspace]\n\n{}\n",
            Self::quoted(&Self::package_name()),
            Self::quoted(BuildPlan::EDITION),
            Self::quoted(&self.bin_name),
            Self::quoted(Self::SOURCE_PATH),
            plan.manifest_profile(),
        );
        Self::write_section(&mut text, "dependencies", self.dependencies);
        Self::write_section(&mut text, "dev-dependencies", self.dev_dependencies);
        text
    }

    fn write_section(text: &mut String, section: &str, crates: &[CrateDependency]) {
        if crates.is_empty() {
            return;
        }
        text.push_str(&format!("[{section}]\n"));
        crates
            .iter()
            .for_each(|dependency| text.push_str(&Self::entry(dependency)));
        text.push('\n');
    }

    fn entry(dependency: &CrateDependency) -> String {
        let mut fields = Vec::new();
        if let Some(version) = &dependency.version {
            fields.push(format!("version = {}", Self::quoted(version)));
        }
        if let Some(path) = &dependency.path {
            fields.push(format!(
                "path = {}",
                Self::quoted(&path.display().to_string())
            ));
        }
        if !dependency.features.is_empty() {
            let features: Vec<String> = dependency
                .features
                .iter()
                .map(|feature| Self::quoted(feature))
                .collect();
            fields.push(format!("features = [{}]", features.join(", ")));
        }
        format!(
            "{} = {{ {} }}\n",
            Self::quoted(&dependency.name),
            fields.join(", ")
        )
    }

    fn quoted(text: &str) -> String {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CargoManifest;
    use crate::build::{BuildPlan, BuildProfile};
    use crate::project::CrateDependency;
    use crate::source::{SourceMap, Span};

    fn dependency(
        name: &str,
        version: Option<&str>,
        features: &[&str],
        path: Option<&str>,
    ) -> CrateDependency {
        let mut sources = SourceMap::default();
        let source = sources.add("project", "(fake)");
        CrateDependency {
            name: name.to_string(),
            name_span: Span::new(source, 1..5),
            version: version.map(str::to_string),
            features: features.iter().map(|feature| feature.to_string()).collect(),
            path: path.map(PathBuf::from),
        }
    }

    #[test]
    fn the_manifest_writes_each_field_of_an_entry() {
        let dependencies = [
            dependency("fake-json", Some("1.2"), &["derive", "std"], None),
            dependency("fake_local", None, &[], Some("/fixtures/fake \"local\"")),
        ];
        let dev_dependencies = [dependency("fake-check", Some("0.3"), &[], None)];
        let manifest = CargoManifest {
            bin_name: "demo-00000000".to_string(),
            dependencies: &dependencies,
            dev_dependencies: &dev_dependencies,
        };
        let text = manifest.render(&BuildPlan::new(BuildProfile::Debug));
        assert!(text.contains("[dependencies]\n\"fake-json\" = { version = \"1.2\", features = [\"derive\", \"std\"] }\n"));
        assert!(text.contains("\"fake_local\" = { path = \"/fixtures/fake \\\"local\\\"\" }\n"));
        assert!(text.contains("[dev-dependencies]\n\"fake-check\" = { version = \"0.3\" }\n"));
        assert!(text.contains("[[bin]]\nname = \"demo-00000000\"\n"));
    }

    #[test]
    fn the_manifest_leaves_out_an_empty_section() {
        let manifest = CargoManifest {
            bin_name: "demo-00000000".to_string(),
            dependencies: &[],
            dev_dependencies: &[],
        };
        let text = manifest.render(&BuildPlan::new(BuildProfile::Release));
        assert!(!text.contains("dependencies]"));
        assert!(text.contains("[profile.dev]\ndebug = false\n"));
    }
}
