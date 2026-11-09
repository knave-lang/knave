pub struct QuotedForm {
    text: String,
    pending_space: bool,
}

impl QuotedForm {
    const STRING_DELIMITER: char = '"';
    const STRING_ESCAPE: char = '\\';
    const COMMENT_START: char = ';';
    const LINE_END: char = '\n';

    pub fn of(source: &str) -> String {
        let mut quoted = Self {
            text: String::new(),
            pending_space: false,
        };
        let mut characters = source.chars();
        while let Some(character) = characters.next() {
            match character {
                Self::STRING_DELIMITER => quoted.copy_string(&mut characters),
                Self::COMMENT_START => quoted.skip_comment(&mut characters),
                blank if blank.is_whitespace() => quoted.pending_space = true,
                other => quoted.push(other),
            }
        }
        quoted.text
    }

    fn push(&mut self, character: char) {
        if self.pending_space && !self.text.is_empty() {
            self.text.push(' ');
        }
        self.pending_space = false;
        self.text.push(character);
    }

    fn copy_string(&mut self, characters: &mut std::str::Chars) {
        self.push(Self::STRING_DELIMITER);
        while let Some(character) = characters.next() {
            self.text.push(character);
            match character {
                Self::STRING_ESCAPE => self.text.extend(characters.next()),
                Self::STRING_DELIMITER => return,
                _ => {}
            }
        }
    }

    fn skip_comment(&mut self, characters: &mut std::str::Chars) {
        characters.find(|character| *character == Self::LINE_END);
        self.pending_space = true;
    }
}

#[cfg(test)]
mod tests {
    use super::QuotedForm;

    #[test]
    fn a_quoted_form_drops_its_comments() {
        assert_eq!(QuotedForm::of("(+ a ; one more\n 1)"), "(+ a 1)");
    }

    #[test]
    fn a_quoted_form_keeps_its_strings_as_written() {
        assert_eq!(
            QuotedForm::of("(str  \"a ; b  \\\" c\"\n   x)"),
            "(str \"a ; b  \\\" c\" x)"
        );
    }
}
