#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeSpacing {
    Written,
    Separated,
}

impl CodeSpacing {
    pub fn breaks_before_form(self, written_blank: bool, joins_import_group: bool) -> bool {
        match self {
            Self::Written => written_blank,
            Self::Separated => !joins_import_group,
        }
    }

    pub fn needs_a_line_per_form(self) -> bool {
        self == Self::Written
    }

    pub fn breaks_after_result(self, written_blank: bool) -> bool {
        match self {
            Self::Written => written_blank,
            Self::Separated => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodeSpacing;

    #[test]
    fn written_spacing_breaks_only_where_the_code_wrote_a_blank_line() {
        assert_eq!(
            (
                CodeSpacing::Written.breaks_before_form(true, false),
                CodeSpacing::Written.breaks_before_form(false, false),
                CodeSpacing::Written.breaks_after_result(false),
                CodeSpacing::Written.needs_a_line_per_form()
            ),
            (true, false, false, true)
        );
    }

    #[test]
    fn separated_spacing_breaks_before_each_form_outside_an_import_group() {
        assert_eq!(
            (
                CodeSpacing::Separated.breaks_before_form(false, false),
                CodeSpacing::Separated.breaks_before_form(true, true),
                CodeSpacing::Separated.breaks_after_result(false),
                CodeSpacing::Separated.needs_a_line_per_form()
            ),
            (true, false, true, false)
        );
    }
}
