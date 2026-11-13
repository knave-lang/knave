use rustc_hash::FxHashSet;

use crate::core::{BindingId, CoreModule};

#[derive(Clone, Debug, Default)]
pub struct WrittenBodies {
    bodies: FxHashSet<BindingId>,
}

impl WrittenBodies {
    pub fn of(modules: &[CoreModule]) -> Self {
        let bodies = modules
            .iter()
            .flat_map(|module| &module.items)
            .flat_map(|item| item.kind.method_bodies())
            .collect();
        Self { bodies }
    }

    pub fn contains(&self, body: BindingId) -> bool {
        self.bodies.contains(&body)
    }

    pub fn iter(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.bodies.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet;

    use super::WrittenBodies;
    use crate::core::BindingId;
    use crate::types::{TypedProgram, TypedSource};

    struct WrittenFixture {
        source: TypedSource,
        typed: TypedProgram,
    }

    impl WrittenFixture {
        const PROGRAM: &str = "
            (trait tagged
              (tag (fn ((ref self)) i64) 1)
              (rank (fn ((ref self)) i64)))
            (defstruct car :year i64)
            (impl tagged car
              (rank (c) (:year c)))
            (defn main () (println (tag (->car :year 1))))";

        fn new() -> Self {
            let source = TypedSource::expand(Self::PROGRAM);
            let typed = source.typed();
            Self { source, typed }
        }

        fn written(&self) -> FxHashSet<BindingId> {
            WrittenBodies::of(&self.source.program().modules)
                .iter()
                .collect()
        }

        fn emitted(&self) -> FxHashSet<BindingId> {
            let traits = self.typed.declarations.traits();
            let impl_bodies = traits
                .impls()
                .iter()
                .flat_map(|implementation| &implementation.methods)
                .map(|method| method.binding);
            let default_bodies = traits
                .definitions()
                .iter()
                .flat_map(|definition| &definition.methods)
                .filter_map(|method| method.default);
            impl_bodies.chain(default_bodies).collect()
        }
    }

    #[test]
    fn lists_exactly_the_method_bodies_that_emit_writes() {
        let fixture = WrittenFixture::new();
        assert_eq!(fixture.written(), fixture.emitted());
    }
}
