use std::rc::Rc;

use crate::rust_prelude::RustPrelude;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(super) struct RustTraitPath {
    named: Rc<str>,
}

impl From<&str> for RustTraitPath {
    fn from(written: &str) -> Self {
        let path = written.strip_prefix(Self::GLOBAL_PREFIX).unwrap_or(written);
        let (name, arguments) =
            path.split_at(path.find(Self::ARGUMENT_OPENERS).unwrap_or(path.len()));
        let named = RustPrelude::trait_path(name).map_or_else(
            || Self::standard_path(path),
            |full| format!("{full}{arguments}"),
        );
        Self {
            named: named.into(),
        }
    }
}

impl RustTraitPath {
    const STANDARD_ROOT: &'static str = "std::";
    const STANDARD_ALIASES: [&'static str; 2] = ["core::", "alloc::"];
    const GLOBAL_PREFIX: &'static str = "::";
    const ARGUMENT_OPENERS: [char; 2] = ['<', '('];

    fn standard_path(path: &str) -> String {
        Self::STANDARD_ALIASES
            .iter()
            .find_map(|alias| path.strip_prefix(alias))
            .map_or_else(
                || path.to_string(),
                |rest| format!("{}{rest}", Self::STANDARD_ROOT),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::RustTraitPath;

    struct PathPair {
        written: &'static str,
        other: &'static str,
    }

    impl PathPair {
        fn names_same_trait(&self) -> bool {
            RustTraitPath::from(self.written) == RustTraitPath::from(self.other)
        }
    }

    #[test]
    fn a_rust_prelude_name_matches_its_full_path() {
        let pair = PathPair {
            written: "PartialEq",
            other: "std::cmp::PartialEq",
        };
        assert!(pair.names_same_trait());
    }

    #[test]
    fn a_core_path_matches_its_std_path() {
        let pair = PathPair {
            written: "core::fmt::Display",
            other: "std::fmt::Display",
        };
        assert!(pair.names_same_trait());
    }

    #[test]
    fn a_global_path_matches_the_same_path() {
        let pair = PathPair {
            written: "::std::fmt::Display",
            other: "std::fmt::Display",
        };
        assert!(pair.names_same_trait());
    }

    #[test]
    fn an_alloc_path_matches_its_std_path() {
        let pair = PathPair {
            written: "alloc::string::ToString",
            other: "std::string::ToString",
        };
        assert!(pair.names_same_trait());
    }

    #[test]
    fn a_rust_prelude_name_with_arguments_matches_its_full_path() {
        let pair = PathPair {
            written: "AsRef<str>",
            other: "std::convert::AsRef<str>",
        };
        assert!(pair.names_same_trait());
    }

    #[test]
    fn a_crate_trait_does_not_match_a_prelude_name() {
        let pair = PathPair {
            written: "crate::rt::Clone",
            other: "Clone",
        };
        assert!(!pair.names_same_trait());
    }
}
