use super::CrateDependency;
use super::problems::ProjectProblems;
use crate::diagnostic::Diagnostic;

#[derive(Clone, Default)]
pub struct CrateSet {
    crates: Vec<CrateDependency>,
}

struct PlainVersion {
    parts: Vec<u64>,
}

impl CrateSet {
    pub fn add(&mut self, dependency: CrateDependency) -> Result<(), Diagnostic> {
        let Some(listed) = self
            .crates
            .iter_mut()
            .find(|listed| listed.name == dependency.name)
        else {
            self.crates.push(dependency);
            return Ok(());
        };
        if listed.same_request(&dependency) {
            return Ok(());
        }
        let version = Self::merged_version(listed, &dependency).ok_or_else(|| {
            ProjectProblems::crate_requests_conflict(
                &dependency.name,
                dependency.name_span,
                listed.name_span,
            )
        })?;
        listed.version = Some(version);
        Self::add_features(listed, &dependency);
        Ok(())
    }

    pub fn into_crates(self) -> Vec<CrateDependency> {
        self.crates
    }

    fn merged_version(listed: &CrateDependency, added: &CrateDependency) -> Option<String> {
        let listed_text = Self::version_without_path(listed)?;
        let added_text = Self::version_without_path(added)?;
        PlainVersion::higher_compatible(listed_text, added_text)
    }

    fn version_without_path(dependency: &CrateDependency) -> Option<&str> {
        dependency.path.is_none().then_some(())?;
        dependency.version.as_deref()
    }

    fn add_features(listed: &mut CrateDependency, added: &CrateDependency) {
        for feature in &added.features {
            if !listed.features.contains(feature) {
                listed.features.push(feature.clone());
            }
        }
        listed.features.sort();
    }

    pub fn count(&self) -> usize {
        self.crates.len()
    }
}

impl PlainVersion {
    const CARET: char = '^';
    const SEPARATOR: char = '.';
    const MOST_PARTS: usize = 3;

    fn higher_compatible(listed_text: &str, added_text: &str) -> Option<String> {
        let (listed, added) = (Self::read(listed_text)?, Self::read(added_text)?);
        listed.is_compatible_with(&added).then_some(())?;
        let highest = if added.is_above(&listed) {
            added_text
        } else {
            listed_text
        };
        Some(Self::without_caret(highest).to_string())
    }

    fn without_caret(text: &str) -> &str {
        text.strip_prefix(Self::CARET).unwrap_or(text)
    }

    fn read(text: &str) -> Option<Self> {
        let parts: Vec<u64> = Self::without_caret(text)
            .split(Self::SEPARATOR)
            .map(|part| part.parse().ok())
            .collect::<Option<_>>()?;
        (parts.len() <= Self::MOST_PARTS).then_some(Self { parts })
    }

    fn is_compatible_with(&self, other: &Self) -> bool {
        self.compatibility_key() == other.compatibility_key()
    }

    fn compatibility_key(&self) -> &[u64] {
        let significant = self
            .parts
            .iter()
            .position(|part| *part != 0)
            .unwrap_or(self.parts.len() - 1);
        &self.parts[..=significant]
    }

    fn is_above(&self, other: &Self) -> bool {
        let padded = |version: &Self| -> Vec<u64> {
            let mut parts = version.parts.clone();
            parts.resize(Self::MOST_PARTS, 0);
            parts
        };
        padded(self) > padded(other)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CrateSet;
    use crate::project::CrateDependency;
    use crate::source::{SourceMap, Span};

    struct Request {
        version: Option<&'static str>,
        features: &'static [&'static str],
        path: Option<&'static str>,
    }

    impl Request {
        fn version(version: &'static str) -> Self {
            Self {
                version: Some(version),
                features: &[],
                path: None,
            }
        }

        fn path(path: &'static str) -> Self {
            Self {
                version: None,
                features: &[],
                path: Some(path),
            }
        }

        fn with_features(self, features: &'static [&'static str]) -> Self {
            Self { features, ..self }
        }

        fn dependency(&self) -> CrateDependency {
            let mut sources = SourceMap::default();
            let source = sources.add("project", "(serde)");
            CrateDependency {
                name: "serde".to_string(),
                name_span: Span::new(source, 1..6),
                version: self.version.map(str::to_string),
                features: self
                    .features
                    .iter()
                    .map(|feature| feature.to_string())
                    .collect(),
                path: self.path.map(PathBuf::from),
            }
        }
    }

    fn merged(first: Request, second: Request) -> Option<CrateDependency> {
        let mut set = CrateSet::default();
        set.add(first.dependency()).ok()?;
        set.add(second.dependency()).ok()?;
        let mut crates = set.into_crates();
        assert_eq!(crates.len(), 1);
        crates.pop()
    }

    fn merged_version(first: &'static str, second: &'static str) -> Option<String> {
        merged(Request::version(first), Request::version(second))?.version
    }

    #[test]
    fn compatible_versions_merge_to_the_highest() {
        assert_eq!(merged_version("1.0", "1.2").as_deref(), Some("1.2"));
        assert_eq!(merged_version("1.4.1", "^1.2").as_deref(), Some("1.4.1"));
        assert_eq!(merged_version("^1.2", "1.0").as_deref(), Some("1.2"));
    }

    #[test]
    fn a_zero_major_version_merges_only_within_its_minor() {
        assert_eq!(merged_version("0.1", "0.1.5").as_deref(), Some("0.1.5"));
        assert_eq!(merged_version("0.1", "0.2"), None);
    }

    #[test]
    fn a_zero_minor_version_merges_only_on_its_patch() {
        assert_eq!(merged_version("0.0.3", "^0.0.3").as_deref(), Some("0.0.3"));
        assert_eq!(merged_version("0.0", "0.0.0"), None);
        assert_eq!(merged_version("0.0.3", "0.0.4"), None);
    }

    #[test]
    fn different_majors_do_not_merge() {
        assert_eq!(merged_version("1.0", "2.0"), None);
    }

    #[test]
    fn a_version_that_is_not_plain_does_not_merge() {
        assert_eq!(merged_version("=1.0", "1.0"), None);
        assert_eq!(merged_version("1.0.0.1", "1.0"), None);
    }

    #[test]
    fn a_path_does_not_merge_with_a_version_or_another_path() {
        assert!(merged(Request::path("/a"), Request::version("1.0")).is_none());
        assert!(merged(Request::path("/a"), Request::path("/b")).is_none());
        assert!(merged(Request::path("/a"), Request::path("/a")).is_some());
    }

    #[test]
    fn merged_features_are_the_union() {
        let first = Request::version("1.0").with_features(&["derive"]);
        let second = Request::version("1.1").with_features(&["rc", "derive"]);
        let dependency = merged(first, second).expect("the requests merge");
        assert_eq!(dependency.features, ["derive", "rc"]);
    }

    #[test]
    fn a_repeated_feature_does_not_hide_a_missing_one() {
        let first = Request::path("/a").with_features(&["a", "a"]);
        let second = Request::path("/a").with_features(&["a", "b"]);
        assert!(merged(first, second).is_none());
        let first = Request::path("/a").with_features(&["a", "b"]);
        let second = Request::path("/a").with_features(&["a", "a"]);
        assert!(merged(first, second).is_none());
    }

    #[test]
    fn one_path_with_features_in_another_order_is_one_request() {
        let first = Request::path("/a").with_features(&["a", "b"]);
        let second = Request::path("/a").with_features(&["b", "a"]);
        assert!(merged(first, second).is_some());
    }
}
