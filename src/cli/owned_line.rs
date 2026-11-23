use std::io;
use std::path::Path;

use super::announced_write::AnnouncedWrite;
use super::exit_status::ExitStatus;
use super::line_upgrade::LineUpgrade;
use super::new_file::NewFile;

pub struct OwnedLine {
    path: &'static str,
    line: &'static str,
    attribute_prefix: &'static str,
}

impl OwnedLine {
    const LF: &str = "\n";
    const CRLF: &str = "\r\n";
    const BYTE_ORDER_MARK: &str = "\u{feff}";

    pub const fn new(
        path: &'static str,
        line: &'static str,
        attribute_prefix: &'static str,
    ) -> Self {
        Self {
            path,
            line,
            attribute_prefix,
        }
    }

    pub fn new_file(&self, root: &Path) -> NewFile {
        NewFile::new(root.join(self.path), format!("{}\n", self.line))
    }

    pub fn pending_write(&self, root: &Path) -> Result<Option<AnnouncedWrite>, ExitStatus> {
        let file = self.new_file(root);
        match file.written_bytes()? {
            None => Ok(Some(AnnouncedWrite::creation(file, Path::new(self.path)))),
            Some(written) => {
                let written = Self::text_of(&file, &written)?;
                Ok(self.upgrade(&written).map(|upgrade| {
                    let text = upgrade.text().to_string();
                    AnnouncedWrite::new(file.with_text(text), upgrade.announcement().to_string())
                }))
            }
        }
    }

    fn text_of(file: &NewFile, written: &[u8]) -> Result<String, ExitStatus> {
        io::read_to_string(written).map_err(|error| file.report_unreadable(&error))
    }

    fn upgrade(&self, written: &str) -> Option<LineUpgrade> {
        let (mark, body) = Self::split_byte_order_mark(written);
        if !body.lines().any(|line| self.owns(line)) {
            let announcement = format!("added the `{}` line to `{}`", self.pattern(), self.path);
            let appended = format!("{mark}{}", self.appended_to(body));
            return Some(LineUpgrade::new(appended, announcement));
        }
        let replaced = format!("{mark}{}", self.replaced_in(body));
        let announcement = format!("rewrote the `{}` line of `{}`", self.pattern(), self.path);
        (replaced != written).then(|| LineUpgrade::new(replaced, announcement))
    }

    fn split_byte_order_mark(written: &str) -> (&str, &str) {
        match written.strip_prefix(Self::BYTE_ORDER_MARK) {
            Some(body) => (Self::BYTE_ORDER_MARK, body),
            None => ("", written),
        }
    }

    fn replaced_in(&self, written: &str) -> String {
        written
            .split_inclusive('\n')
            .map(|line| {
                let content = line.trim_end_matches(['\n', '\r']);
                if self.owns(content) {
                    format!("{}{}", self.line, &line[content.len()..])
                } else {
                    line.to_string()
                }
            })
            .collect()
    }

    fn appended_to(&self, written: &str) -> String {
        let ending = Self::line_ending_of(written);
        let separator = if written.is_empty() || written.ends_with('\n') {
            ""
        } else {
            ending
        };
        format!("{written}{separator}{}{ending}", self.line)
    }

    fn line_ending_of(written: &str) -> &'static str {
        match written.split_once('\n') {
            Some((first, _)) if first.ends_with('\r') => Self::CRLF,
            _ => Self::LF,
        }
    }

    fn owns(&self, line: &str) -> bool {
        let mut fields = line.split_whitespace();
        fields.next() == Some(self.pattern())
            && fields.any(|attribute| attribute.starts_with(self.attribute_prefix))
    }

    fn pattern(&self) -> &str {
        self.line.split_whitespace().next().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::OwnedLine;

    const LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";
    const OWNED: OwnedLine = OwnedLine::new(".gitattributes", LINE, "linguist-");

    struct LinguistFixture;

    impl LinguistFixture {
        fn upgraded(written: &str) -> String {
            OWNED
                .upgrade(written)
                .map_or_else(|| written.to_string(), |upgrade| upgrade.text().to_string())
        }

        fn is_appended(written: &str) -> bool {
            OWNED
                .upgrade(written)
                .is_some_and(|upgrade| upgrade.announcement().starts_with("added"))
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
        assert!(OWNED.upgrade(&format!("{LINE}\n")).is_none());
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
    fn a_crlf_file_takes_the_line_with_crlf() {
        assert_eq!(
            LinguistFixture::upgraded("*.png binary\r\n*.txt text"),
            format!("*.png binary\r\n*.txt text\r\n{LINE}\r\n")
        );
    }

    #[test]
    fn an_owned_line_after_a_byte_order_mark_is_replaced_and_keeps_the_mark() {
        assert_eq!(
            LinguistFixture::upgraded("\u{feff}*.knv linguist-language=Lisp\n"),
            format!("\u{feff}{LINE}\n")
        );
    }

    #[test]
    fn a_current_line_after_a_byte_order_mark_needs_no_upgrade() {
        assert!(OWNED.upgrade(&format!("\u{feff}{LINE}\n")).is_none());
    }

    #[test]
    fn an_empty_file_takes_the_line_alone() {
        assert_eq!(LinguistFixture::upgraded(""), format!("{LINE}\n"));
    }
}
