use rustc_hash::FxHashMap;

use crate::core::{BindingArena, BindingId};

#[derive(Default)]
pub struct SignatureDeclarations {
    pub(super) prelude_definitions: Vec<BindingId>,
    pub(super) written_parameters: FxHashMap<BindingId, Vec<bool>>,
}

impl SignatureDeclarations {
    pub fn prelude_defines_value(&self, name: &str, bindings: &BindingArena) -> bool {
        self.prelude_definitions
            .iter()
            .any(|binding| bindings.get(*binding).name() == name)
    }

    pub fn written_at(&self, callee: BindingId, position: usize) -> bool {
        self.written_parameters
            .get(&callee)
            .and_then(|written| written.get(position))
            .copied()
            .unwrap_or(true)
    }

    pub fn written_parameter_count(&self, callee: BindingId) -> usize {
        self.written_parameters.get(&callee).map_or(0, Vec::len)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::declarations_fixture::DeclarationsFixture;

    struct SignatureFixture;

    impl SignatureFixture {
        const SOURCE: &str = "
            (defn pick ((left i64) right) left)
            (defn main () (do))";
        const WRITTEN: usize = 0;
        const UNWRITTEN: usize = 1;
        const PARAMETERS: usize = 2;
        const PRELUDE_VALUE: &str = "sum";
    }

    #[test]
    fn a_parameter_with_a_written_type_is_written() {
        let fixture = DeclarationsFixture::new(SignatureFixture::SOURCE);
        let pick = fixture.function_named("pick");
        assert!(
            fixture
                .signatures()
                .written_at(pick, SignatureFixture::WRITTEN)
        );
    }

    #[test]
    fn a_parameter_without_a_written_type_is_not_written() {
        let fixture = DeclarationsFixture::new(SignatureFixture::SOURCE);
        let pick = fixture.function_named("pick");
        assert!(
            !fixture
                .signatures()
                .written_at(pick, SignatureFixture::UNWRITTEN)
        );
    }

    #[test]
    fn a_function_with_no_recorded_parameters_counts_them_as_written() {
        let fixture = DeclarationsFixture::new(SignatureFixture::SOURCE);
        let main = fixture.function_named("main");
        assert!(
            fixture
                .signatures()
                .written_at(main, SignatureFixture::WRITTEN)
        );
    }

    #[test]
    fn the_written_parameters_are_counted() {
        let fixture = DeclarationsFixture::new(SignatureFixture::SOURCE);
        let pick = fixture.function_named("pick");
        assert_eq!(
            fixture.signatures().written_parameter_count(pick),
            SignatureFixture::PARAMETERS
        );
    }

    #[test]
    fn the_prelude_defines_its_values() {
        let fixture = DeclarationsFixture::new(SignatureFixture::SOURCE);
        assert!(
            fixture
                .signatures()
                .prelude_defines_value(SignatureFixture::PRELUDE_VALUE, fixture.bindings())
        );
    }

    #[test]
    fn the_prelude_does_not_define_a_user_value() {
        let fixture = DeclarationsFixture::new(SignatureFixture::SOURCE);
        assert!(
            !fixture
                .signatures()
                .prelude_defines_value("pick", fixture.bindings())
        );
    }
}
