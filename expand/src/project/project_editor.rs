use std::ops::Range;

use super::dependency_reader::{DependencyReader, Section};
use super::reader::ProjectReader;
use crate::reader::Form;

pub struct CrateEntry {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

pub struct ProjectEditor<'t> {
    text: &'t str,
    project: &'t Form,
}

impl CrateEntry {
    fn text(&self) -> String {
        let mut text = format!("({} \"{}\"", self.name, self.version);
        if !self.features.is_empty() {
            let features: Vec<String> = self
                .features
                .iter()
                .map(|feature| format!("\"{feature}\""))
                .collect();
            text.push_str(&format!(" :features ({})", features.join(" ")));
        }
        text.push(')');
        text
    }
}

impl<'t> ProjectEditor<'t> {
    const LINE_SPACE: [char; 2] = [' ', '\t'];
    const NEWLINE: char = '\n';
    const LINE_BREAKS: [&'static str; 2] = ["\r\n", "\n"];

    pub fn of(text: &'t str, forms: &'t [Form]) -> Option<Self> {
        let project = forms
            .iter()
            .find(|form| Self::head_named(form, ProjectReader::HEAD))?;
        Some(Self { text, project })
    }

    pub fn with_crate(&self, section: Section, entry: &CrateEntry) -> String {
        let Some(section_form) = self.section(section) else {
            let new_section = format!("\n({} {})", section.name(), entry.text());
            return self.splice(Self::before_close(self.project), &new_section);
        };
        match Self::entry_named(section_form, &entry.name) {
            Some(listed) => self.splice(listed.span().range(), &entry.text()),
            None => self.splice(
                Self::before_close(section_form),
                &format!(" {}", entry.text()),
            ),
        }
    }

    pub fn without_crate(&self, section: Section, name: &str) -> Option<String> {
        let section_form = self.section(section)?;
        let listed = Self::entry_named(section_form, name)?;
        let only_entry = Self::entries(section_form).len() == 1;
        let removed = if only_entry { section_form } else { listed };
        Some(self.splice(self.with_leading_space(removed), ""))
    }

    fn section(&self, section: Section) -> Option<&'t Form> {
        Self::entries(self.project)
            .iter()
            .find(|form| Self::head_named(form, section.name()))
    }

    fn entry_named<'f>(section_form: &'f Form, name: &str) -> Option<&'f Form> {
        Self::entries(section_form).iter().find(|entry| {
            entry
                .as_list()
                .and_then(|parts| parts.first())
                .and_then(DependencyReader::bare_name)
                == Some(name)
        })
    }

    fn entries(form: &Form) -> &[Form] {
        form.as_list()
            .map_or(&[], |parts| parts.get(1..).unwrap_or(&[]))
    }

    fn head_named(form: &Form, name: &str) -> bool {
        form.as_list()
            .and_then(|parts| parts.first())
            .and_then(|head| head.as_symbol())
            .is_some_and(|head| head.name() == name)
    }

    fn before_close(form: &Form) -> Range<usize> {
        let close = form.span().range().end - 1;
        close..close
    }

    fn with_leading_space(&self, form: &Form) -> Range<usize> {
        let range = form.span().range();
        let start = self.text[..range.start]
            .trim_end_matches(Self::LINE_SPACE)
            .len();
        let after = &self.text[range.end..];
        let line_end = range.end + (after.len() - after.trim_start_matches(Self::LINE_SPACE).len());
        let starts_line = start == 0 || self.text[..start].ends_with(Self::NEWLINE);
        let line_break = Self::LINE_BREAKS
            .iter()
            .find(|line_break| self.text[line_end..].starts_with(*line_break))
            .map_or(0, |line_break| line_break.len());
        if starts_line && line_break > 0 {
            return start..line_end + line_break;
        }
        start..range.end
    }
    fn splice(&self, range: Range<usize>, replacement: &str) -> String {
        format!(
            "{}{replacement}{}",
            &self.text[..range.start],
            &self.text[range.end..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{CrateEntry, ProjectEditor};
    use crate::project::Section;
    use crate::reader::Reader;
    use crate::source::SourceMap;

    fn edited(text: &str, edit: impl Fn(&ProjectEditor<'_>) -> Option<String>) -> Option<String> {
        let mut sources = SourceMap::default();
        let source = sources.add("project", text);
        let forms = Reader::new(sources.file(source))
            .read()
            .expect("the text reads");
        edit(&ProjectEditor::of(text, &forms).expect("the text holds a project"))
    }

    fn entry(features: &[&str]) -> CrateEntry {
        CrateEntry {
            name: "fake-json".to_string(),
            version: "1.4.2".to_string(),
            features: features.iter().map(|feature| feature.to_string()).collect(),
        }
    }

    #[test]
    fn adding_to_a_project_without_the_section_creates_it() {
        let text = edited("(defproject demo)\n", |editor| {
            Some(editor.with_crate(Section::Dependencies, &entry(&["derive"])))
        });
        assert_eq!(
            text.as_deref(),
            Some(
                "(defproject demo\n(dependencies (fake-json \"1.4.2\" :features (\"derive\"))))\n"
            )
        );
    }

    #[test]
    fn adding_to_a_section_appends_the_entry() {
        let text = edited(
            "(defproject demo (dependencies (other \"1.0\")))\n",
            |editor| Some(editor.with_crate(Section::Dependencies, &entry(&[]))),
        );
        assert_eq!(
            text.as_deref(),
            Some("(defproject demo (dependencies (other \"1.0\") (fake-json \"1.4.2\")))\n")
        );
    }

    #[test]
    fn adding_a_listed_crate_replaces_its_entry() {
        let text = edited(
            "(defproject demo (dev-dependencies (fake-json \"1.0\")))\n",
            |editor| Some(editor.with_crate(Section::DevDependencies, &entry(&[]))),
        );
        assert_eq!(
            text.as_deref(),
            Some("(defproject demo (dev-dependencies (fake-json \"1.4.2\")))\n")
        );
    }

    #[test]
    fn removing_one_of_several_entries_keeps_the_section() {
        let text = edited(
            "(defproject demo (dependencies (other \"1.0\") (fake-json \"1.4.2\")))\n",
            |editor| editor.without_crate(Section::Dependencies, "fake-json"),
        );
        assert_eq!(
            text.as_deref(),
            Some("(defproject demo (dependencies (other \"1.0\")))\n")
        );
    }

    #[test]
    fn removing_the_last_entry_removes_the_section() {
        let text = edited(
            "(defproject demo (dependencies (fake-json \"1.4.2\")))\n",
            |editor| editor.without_crate(Section::Dependencies, "fake-json"),
        );
        assert_eq!(text.as_deref(), Some("(defproject demo)\n"));
    }

    #[test]
    fn removing_an_entry_after_a_comment_keeps_the_comment() {
        let text = edited(
            "(defproject demo\n  (dependencies ; first\n    (other \"1.0\")\n    ; json\n    (fake-json \"1.4.2\")))\n",
            |editor| editor.without_crate(Section::Dependencies, "fake-json"),
        );
        assert_eq!(
            text.as_deref(),
            Some(
                "(defproject demo\n  (dependencies ; first\n    (other \"1.0\")\n    ; json\n))\n"
            )
        );
    }

    #[test]
    fn removing_a_section_after_a_comment_keeps_the_comment() {
        let text = edited(
            "(defproject demo ; the demo\n  (dependencies (fake-json \"1.4.2\")))\n",
            |editor| editor.without_crate(Section::Dependencies, "fake-json"),
        );
        assert_eq!(text.as_deref(), Some("(defproject demo ; the demo\n)\n"));
    }

    #[test]
    fn removing_the_entry_after_a_section_comment_keeps_the_comment() {
        let text = edited(
            "(defproject demo\n  (dependencies ; crates\n    (other \"1.0\")\n    (fake-json \"1.4.2\")))\n",
            |editor| editor.without_crate(Section::Dependencies, "other"),
        );
        assert_eq!(
            text.as_deref(),
            Some("(defproject demo\n  (dependencies ; crates\n    (fake-json \"1.4.2\")))\n")
        );
    }

    #[test]
    fn removing_an_entry_on_its_own_line_removes_a_crlf_line() {
        let text = edited(
            "(defproject demo\r\n  (dependencies\r\n    (other \"1.0\")\r\n    (fake-json \"1.4.2\")))\r\n",
            |editor| editor.without_crate(Section::Dependencies, "other"),
        );
        assert_eq!(
            text.as_deref(),
            Some("(defproject demo\r\n  (dependencies\r\n    (fake-json \"1.4.2\")))\r\n")
        );
    }

    #[test]
    fn removing_an_unlisted_crate_changes_nothing() {
        let text = edited(
            "(defproject demo (dependencies (other \"1.0\")))\n",
            |editor| editor.without_crate(Section::Dependencies, "fake-json"),
        );
        assert_eq!(text, None);
        let text = edited("(defproject demo)\n", |editor| {
            editor.without_crate(Section::Dependencies, "fake-json")
        });
        assert_eq!(text, None);
    }
}
