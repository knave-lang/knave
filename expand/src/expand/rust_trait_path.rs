pub(super) struct RustTraitPath<'p> {
    written: &'p str,
}

impl<'p> RustTraitPath<'p> {
    const STANDARD_ROOT: &'static str = "std::";
    const STANDARD_ALIASES: [&'static str; 2] = ["core::", "alloc::"];
    const GLOBAL_PREFIX: &'static str = "::";
    const RUST_PRELUDE_TRAITS: [(&'static str, &'static str); 31] = [
        ("AsMut", "std::convert::AsMut"),
        ("AsRef", "std::convert::AsRef"),
        ("Clone", "std::clone::Clone"),
        ("Copy", "std::marker::Copy"),
        ("Default", "std::default::Default"),
        ("DoubleEndedIterator", "std::iter::DoubleEndedIterator"),
        ("Drop", "std::ops::Drop"),
        ("Eq", "std::cmp::Eq"),
        ("ExactSizeIterator", "std::iter::ExactSizeIterator"),
        ("Extend", "std::iter::Extend"),
        ("Fn", "std::ops::Fn"),
        ("FnMut", "std::ops::FnMut"),
        ("FnOnce", "std::ops::FnOnce"),
        ("From", "std::convert::From"),
        ("FromIterator", "std::iter::FromIterator"),
        ("Future", "std::future::Future"),
        ("Into", "std::convert::Into"),
        ("IntoFuture", "std::future::IntoFuture"),
        ("IntoIterator", "std::iter::IntoIterator"),
        ("Iterator", "std::iter::Iterator"),
        ("Ord", "std::cmp::Ord"),
        ("PartialEq", "std::cmp::PartialEq"),
        ("PartialOrd", "std::cmp::PartialOrd"),
        ("Send", "std::marker::Send"),
        ("Sized", "std::marker::Sized"),
        ("Sync", "std::marker::Sync"),
        ("ToOwned", "std::borrow::ToOwned"),
        ("ToString", "std::string::ToString"),
        ("TryFrom", "std::convert::TryFrom"),
        ("TryInto", "std::convert::TryInto"),
        ("Unpin", "std::marker::Unpin"),
    ];

    pub(super) fn new(written: &'p str) -> Self {
        Self { written }
    }

    pub(super) fn names_same_trait(&self, other: &RustTraitPath<'_>) -> bool {
        self.resolved() == other.resolved()
    }

    fn resolved(&self) -> String {
        let path = self
            .written
            .strip_prefix(Self::GLOBAL_PREFIX)
            .unwrap_or(self.written);
        if let Some((_, full)) = Self::RUST_PRELUDE_TRAITS
            .iter()
            .find(|(name, _)| *name == path)
        {
            return (*full).to_string();
        }
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
            RustTraitPath::new(self.written).names_same_trait(&RustTraitPath::new(self.other))
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
    fn a_crate_trait_does_not_match_a_prelude_name() {
        let pair = PathPair {
            written: "crate::rt::Show",
            other: "Show",
        };
        assert!(!pair.names_same_trait());
    }
}
