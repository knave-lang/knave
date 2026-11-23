use std::path::Path;

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

    pub fn upgraded(&self, written: &str) -> String {
        if written.lines().any(Self::owns) {
            Self::replaced_in(written)
        } else {
            Self::appended_to(written)
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

    const LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";

    #[test]
    fn each_owned_line_takes_the_line_and_keeps_its_line_ending() {
        let written = "*.knv linguist-language=Lisp\r\n*.png binary\n*.knv -linguist-detectable linguist-generated";
        assert_eq!(
            GithubLinguist.upgraded(written),
            format!("{LINE}\r\n*.png binary\n{LINE}")
        );
    }

    #[test]
    fn a_line_of_another_pattern_is_not_owned() {
        assert_eq!(
            GithubLinguist.upgraded("*.rs linguist-language=Rust\n"),
            format!("*.rs linguist-language=Rust\n{LINE}\n")
        );
    }

    #[test]
    fn a_line_of_the_pattern_without_a_linguist_attribute_is_not_owned() {
        assert_eq!(
            GithubLinguist.upgraded("*.knv text\n"),
            format!("*.knv text\n{LINE}\n")
        );
    }

    #[test]
    fn an_empty_file_takes_the_line_alone() {
        assert_eq!(GithubLinguist.upgraded(""), format!("{LINE}\n"));
    }
}
