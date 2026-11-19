use std::fmt;

use crate::language::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl ReleaseVersion {
    const SEPARATOR: char = '.';

    pub fn running() -> Self {
        Self::parse(Language::VERSION).expect("the compiler's version is a release version")
    }

    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<u64> = text
            .split(Self::SEPARATOR)
            .map(Self::number)
            .collect::<Option<_>>()?;
        let [major, minor, patch] = parts[..] else {
            return None;
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn number(text: &str) -> Option<u64> {
        let digits_only = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
        digits_only.then(|| text.parse().ok()).flatten()
    }
}

impl fmt::Display for ReleaseVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::ReleaseVersion;

    #[test]
    fn a_release_version_reads_three_numbers() {
        let version = ReleaseVersion::parse("0.18.2").expect("the version reads");
        assert_eq!(version.to_string(), "0.18.2");
    }

    #[test]
    fn a_release_version_compares_each_number_in_turn() {
        let ordered =
            ["0.9.9", "0.18.0", "0.18.1", "1.0.0"].map(|text| ReleaseVersion::parse(text));
        assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_release_version_refuses_any_other_shape() {
        for text in [
            "",
            "0.18",
            "0.18.0.1",
            "0.18.x",
            "0.18.-1",
            "0..1",
            "v0.18.0",
            "0.18.0-rc.1",
        ] {
            assert_eq!(ReleaseVersion::parse(text), None, "`{text}` reads");
        }
    }

    #[test]
    fn the_running_compiler_has_a_release_version() {
        assert_eq!(
            ReleaseVersion::running().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
