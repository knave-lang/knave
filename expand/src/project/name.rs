use super::problems::ProjectProblems;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub struct ProjectName;

impl ProjectName {
    const WORD_SEPARATOR: char = '-';

    pub fn check(name: &str, span: Span) -> Result<(), Diagnostic> {
        if Self::is_kebab_case(name) {
            return Ok(());
        }
        Err(ProjectProblems::invalid_project_name(name, span))
    }

    pub fn is_kebab_case(name: &str) -> bool {
        let starts_with_letter = name.starts_with(|first: char| first.is_ascii_lowercase());
        starts_with_letter && name.split(Self::WORD_SEPARATOR).all(Self::is_kebab_word)
    }

    fn is_kebab_word(word: &str) -> bool {
        !word.is_empty()
            && word
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    }
}
