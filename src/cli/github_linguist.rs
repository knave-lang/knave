use std::path::Path;

use super::linguist_upgrade::LinguistUpgrade;
use super::new_file::NewFile;

pub struct GithubLinguist;

impl GithubLinguist {
    pub const PATH: &str = ".gitattributes";
    pub const PATTERN: &str = "*.knv";
    const ATTRIBUTE_PREFIX: &str = "linguist-";
    const LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";

    pub fn new_file(&self, root: &Path) -> NewFile {
        NewFile::new(root.join(Self::PATH), format!("{}\n", Self::LINE))
    }

    pub fn upgrade(&self, written: &str) -> Option<LinguistUpgrade> {
        if !written.lines().any(Self::owns) {
            return Some(LinguistUpgrade::Appended(Self::appended_to(written)));
        }
        let replaced = Self::replaced_in(written);
        (replaced != written).then_some(LinguistUpgrade::Replaced(replaced))
    }

    pub fn announcement(&self, upgrade: &LinguistUpgrade) -> String {
        match upgrade {
            LinguistUpgrade::Appended(_) => {
                format!("added the `{}` line to `{}`", Self::PATTERN, Self::PATH)
            }
            LinguistUpgrade::Replaced(_) => {
                format!("rewrote the `{}` line of `{}`", Self::PATTERN, Self::PATH)
            }
        }
    }

    fn replaced_in(written: &str) -> String {
        written
            .split_inclusive('\n')
            .map(|line| {
                let content = line.trim_end_matches(['\n', '\r']);
                if Self::owns(content) {
                    format!("{}{}", Self::LINE, &line[content.len()..])
                } else {
                    line.to_string()
                }
            })
            .collect()
    }

    fn appended_to(written: &str) -> String {
        let separator = if written.is_empty() || written.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        format!("{written}{separator}{}\n", Self::LINE)
    }

    fn owns(line: &str) -> bool {
        let mut fields = line.split_whitespace();
        fields.next() == Some(Self::PATTERN)
            && fields.any(|attribute| attribute.starts_with(Self::ATTRIBUTE_PREFIX))
    }
}

#[cfg(test)]
mod tests {
    use super::GithubLinguist;
    use crate::cli::linguist_upgrade::LinguistUpgrade;

    const LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";

    struct LinguistFixture;

    impl LinguistFixture {
        fn upgraded(written: &str) -> String {
            GithubLinguist
                .upgrade(written)
                .map_or_else(|| written.to_string(), |upgrade| upgrade.text().to_string())
        }

        fn is_appended(written: &str) -> bool {
            matches!(
                GithubLinguist.upgrade(written),
                Some(LinguistUpgrade::Appended(_))
            )
        }
    }

    #[test]
    fn each_owned_line_takes_the_line_and_keeps_its_line_ending() {
        let written = "*.knv linguist-language=Lisp\r\n*.png binary\n*.knv -linguist-detectable linguist-generated";
        assert_eq!(
            LinguistFixture::upgraded(written),
            format!("{LINE}\r\n*.png binary\n{LINE}")
        );
        assert!(!LinguistFixture::is_appended(written));
    }

    #[test]
    fn a_current_line_needs_no_upgrade() {
        assert!(GithubLinguist.upgrade(&format!("{LINE}\n")).is_none());
    }

    #[test]
    fn a_line_of_another_pattern_is_not_owned() {
        let written = "*.rs linguist-language=Rust\n";
        assert_eq!(
            LinguistFixture::upgraded(written),
            format!("*.rs linguist-language=Rust\n{LINE}\n")
        );
        assert!(LinguistFixture::is_appended(written));
    }

    #[test]
    fn a_line_of_the_pattern_without_a_linguist_attribute_is_not_owned() {
        assert_eq!(
            LinguistFixture::upgraded("*.knv text\n"),
            format!("*.knv text\n{LINE}\n")
        );
    }

    #[test]
    fn an_empty_file_takes_the_line_alone() {
        assert_eq!(LinguistFixture::upgraded(""), format!("{LINE}\n"));
    }
}
