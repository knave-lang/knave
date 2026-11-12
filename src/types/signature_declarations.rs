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
    use rustc_hash::FxHashMap;

    use super::SignatureDeclarations;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId};

    struct SignatureFixture {
        bindings: BindingArena,
        signatures: SignatureDeclarations,
    }

    impl SignatureFixture {
        const WRITTEN: usize = 0;
        const UNWRITTEN: usize = 1;
        const PRELUDE_VALUE: &str = "sum";
        const USER_VALUE: &str = "pick";

        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let sum = Self::global(&mut bindings, Self::PRELUDE_VALUE);
            let pick = Self::global(&mut bindings, Self::USER_VALUE);
            let signatures = SignatureDeclarations {
                prelude_definitions: vec![sum],
                written_parameters: FxHashMap::from_iter([(pick, vec![true, false])]),
            };
            Self {
                bindings,
                signatures,
            }
        }

        fn global(bindings: &mut BindingArena, name: &str) -> BindingId {
            bindings.add(BindingInfo::new(
                name.into(),
                BindingKind::Global(ModuleId::new(0)),
                None,
            ))
        }

        fn pick(&self) -> BindingId {
            self.named(Self::USER_VALUE)
        }

        fn sum(&self) -> BindingId {
            self.named(Self::PRELUDE_VALUE)
        }

        fn named(&self, name: &str) -> BindingId {
            self.bindings
                .ids()
                .find(|binding| self.bindings.get(*binding).name() == name)
                .expect("the fixture adds the name")
        }
    }

    #[test]
    fn a_parameter_with_a_written_type_is_written() {
        let fixture = SignatureFixture::new();
        assert!(
            fixture
                .signatures
                .written_at(fixture.pick(), SignatureFixture::WRITTEN)
        );
    }

    #[test]
    fn a_parameter_without_a_written_type_is_not_written() {
        let fixture = SignatureFixture::new();
        assert!(
            !fixture
                .signatures
                .written_at(fixture.pick(), SignatureFixture::UNWRITTEN)
        );
    }

    #[test]
    fn a_function_with_no_recorded_parameters_counts_them_as_written() {
        let fixture = SignatureFixture::new();
        assert!(
            fixture
                .signatures
                .written_at(fixture.sum(), SignatureFixture::WRITTEN)
        );
    }

    #[test]
    fn the_written_parameters_are_counted() {
        let fixture = SignatureFixture::new();
        assert_eq!(
            fixture.signatures.written_parameter_count(fixture.pick()),
            2
        );
    }

    #[test]
    fn the_prelude_defines_its_values() {
        let fixture = SignatureFixture::new();
        assert!(
            fixture
                .signatures
                .prelude_defines_value(SignatureFixture::PRELUDE_VALUE, &fixture.bindings)
        );
    }

    #[test]
    fn the_prelude_does_not_define_a_user_value() {
        let fixture = SignatureFixture::new();
        assert!(
            !fixture
                .signatures
                .prelude_defines_value(SignatureFixture::USER_VALUE, &fixture.bindings)
        );
    }
}
