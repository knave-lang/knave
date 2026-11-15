#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveNames {
    spelling: &'static str,
    shown: &'static str,
    rust: &'static str,
}

impl PrimitiveNames {
    pub const fn same(spelling: &'static str) -> Self {
        Self {
            spelling,
            shown: spelling,
            rust: spelling,
        }
    }

    pub const fn emitted_as(self, rust: &'static str) -> Self {
        Self { rust, ..self }
    }

    pub fn shown(self) -> &'static str {
        self.shown
    }

    pub fn spelling(self) -> &'static str {
        self.spelling
    }

    pub fn rust(self) -> &'static str {
        self.rust
    }

    pub fn differing_spelling(self) -> Option<&'static str> {
        (self.spelling != self.shown).then_some(self.spelling)
    }
}

#[cfg(test)]
mod tests {
    use super::PrimitiveNames;
    use crate::core::PrimitiveType;

    struct NamesFixture;

    impl NamesFixture {
        fn shown_as(spelling: &'static str, shown: &'static str) -> PrimitiveNames {
            PrimitiveNames {
                shown,
                ..PrimitiveNames::same(spelling)
            }
        }
    }

    #[test]
    fn names_shown_by_another_name_offer_a_differing_spelling() {
        assert_eq!(
            NamesFixture::shown_as("i64", "int").differing_spelling(),
            Some("i64")
        );
    }

    #[test]
    fn names_shown_by_the_spelling_have_no_differing_spelling() {
        assert_eq!(PrimitiveNames::same("i64").differing_spelling(), None);
    }

    #[test]
    fn a_string_and_unit_emit_their_rust_names() {
        assert_eq!(PrimitiveType::String.names().rust(), "String");
        assert_eq!(PrimitiveType::Unit.names().rust(), "()");
    }

    #[test]
    fn a_string_and_unit_keep_their_kernel_spellings() {
        assert_eq!(PrimitiveType::String.names().spelling(), "string");
        assert_eq!(PrimitiveType::Unit.names().spelling(), "unit");
    }

    #[test]
    fn a_number_emits_its_spelling() {
        assert_eq!(PrimitiveType::I64.names().rust(), "i64");
        assert_eq!(PrimitiveType::F64.names().rust(), "f64");
    }

    #[test]
    fn a_number_shows_its_spelling() {
        assert_eq!(PrimitiveType::I64.names().shown(), "i64");
    }
}
