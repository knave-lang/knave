use super::import_join::ImportJoin;
use super::written_gap::WrittenGap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeSpacing {
    Written,
    Separated,
}

impl CodeSpacing {
    pub fn breaks_before_form(self, written: WrittenGap, join: ImportJoin) -> bool {
        match self {
            Self::Written => written == WrittenGap::BlankLine,
            Self::Separated => join == ImportJoin::Apart,
        }
    }

    pub fn needs_a_line_per_form(self) -> bool {
        self == Self::Written
    }

    pub fn breaks_after_result(self, written: WrittenGap) -> bool {
        match self {
            Self::Written => written == WrittenGap::BlankLine,
            Self::Separated => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeSpacing, ImportJoin, WrittenGap};

    #[test]
    fn written_spacing_breaks_only_where_the_code_wrote_a_blank_line() {
        let spacing = CodeSpacing::Written;
        assert_eq!(
            (
                spacing.breaks_before_form(WrittenGap::BlankLine, ImportJoin::JoinsGroup),
                spacing.breaks_before_form(WrittenGap::LineBreak, ImportJoin::Apart),
                spacing.breaks_after_result(WrittenGap::LineBreak),
                spacing.breaks_after_result(WrittenGap::BlankLine),
                spacing.needs_a_line_per_form()
            ),
            (true, false, false, true, true)
        );
    }

    #[test]
    fn separated_spacing_breaks_before_each_form_outside_an_import_group() {
        let spacing = CodeSpacing::Separated;
        assert_eq!(
            (
                spacing.breaks_before_form(WrittenGap::LineBreak, ImportJoin::Apart),
                spacing.breaks_before_form(WrittenGap::BlankLine, ImportJoin::JoinsGroup),
                spacing.breaks_after_result(WrittenGap::LineBreak),
                spacing.needs_a_line_per_form()
            ),
            (true, false, true, false)
        );
    }
}
