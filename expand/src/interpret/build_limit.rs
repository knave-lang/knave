use super::built_value::BuiltValue;
use super::builtin_site::BuiltinSite;
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub struct BuildLimit {
    site: BuiltinSite,
}

impl BuildLimit {
    pub fn of(function: &'static str, span: Span) -> Self {
        Self {
            site: BuiltinSite { function, span },
        }
    }

    pub fn span(&self) -> Span {
        self.site.span
    }

    #[inline]
    pub fn check<V: BuiltValue>(&self, value: V) -> Result<V, Diagnostic> {
        let size = value.built_size();
        if size <= V::LIMIT {
            Ok(value)
        } else {
            Err(V::too_large(self.site, size))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::BuildLimit;
    use crate::diagnostic::DiagnosticCode;
    use crate::interpret::limited_text::LimitedText;
    use crate::interpret::planned_list::PlannedList;
    use crate::interpret::value_limits::ValueLimits;
    use crate::source::{SourceId, Span};

    struct BuildLimitFixture;

    impl BuildLimitFixture {
        fn limit() -> BuildLimit {
            BuildLimit::of("string", Span::new(SourceId::new(0), 0..1))
        }

        fn text_of(bytes: usize) -> String {
            "x".repeat(bytes)
        }
    }

    #[test]
    fn text_at_the_limit_is_built() {
        let text = BuildLimitFixture::text_of(ValueLimits::TEXT_BYTES);
        assert!(BuildLimitFixture::limit().check(text).is_ok());
    }

    #[test]
    fn text_past_the_limit_is_refused() {
        let text = BuildLimitFixture::text_of(ValueLimits::TEXT_BYTES + 1);
        let refusal = BuildLimitFixture::limit()
            .check(text)
            .expect_err("too long");
        assert_eq!(refusal.code(), Some(&DiagnosticCode::MACRO_VALUE_SIZE));
    }

    #[test]
    fn limited_text_keeps_what_fits() {
        let mut text = LimitedText::default();
        text.write_str("ab").expect("it fits");
        let built = BuildLimitFixture::limit().check(text).expect("it fits");
        assert_eq!(built.into_text(), "ab");
    }

    #[test]
    fn limited_text_stops_writing_past_the_limit() {
        let mut text = LimitedText::default();
        text.write_str(&BuildLimitFixture::text_of(ValueLimits::TEXT_BYTES))
            .expect("it fits");
        assert!(text.write_str("c").is_err());
        assert!(BuildLimitFixture::limit().check(text).is_err());
    }

    #[test]
    fn a_planned_list_past_the_limit_is_refused() {
        let limit = BuildLimitFixture::limit();
        assert!(limit.check(PlannedList(ValueLimits::LIST_VALUES)).is_ok());
        assert!(
            limit
                .check(PlannedList(ValueLimits::LIST_VALUES + 1))
                .is_err()
        );
    }
}
