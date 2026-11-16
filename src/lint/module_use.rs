use rustc_hash::FxHashSet;

use super::binding_owners::BindingOwners;
use super::linted_module::LintedModule;
use crate::core::{BindingArena, BindingId, BindingKind, ImportedName, NamedTypes, References};
use crate::expand::Program;

pub struct ModuleUse<'p> {
    bindings: &'p BindingArena,
    used: FxHashSet<BindingId>,
}

impl<'p> ModuleUse<'p> {
    pub fn of(program: &'p Program, owners: &BindingOwners, linted: &LintedModule) -> Self {
        let bindings = &program.bindings;
        let module = linted.module();
        let mut used: FxHashSet<BindingId> = module.exports().bindings().collect();
        used.extend(
            linted
                .reads()
                .map(|binding| owners.owner(binding, bindings)),
        );
        used.extend(linted.reads());
        for named in program.compile_time_roots.named_by_macros_in(module.id) {
            used.extend([named, owners.owner(named, bindings)]);
        }
        for item in &module.items {
            NamedTypes::of(&item.kind).visit(&mut |binding| {
                used.insert(binding);
            });
        }
        for (_, value) in module.definitions() {
            let references = References::of(value, bindings);
            used.extend(references.written_types().named().iter().copied());
        }
        Self { bindings, used }
    }

    pub fn refers_to(&self, name: &ImportedName) -> bool {
        name.bindings
            .iter()
            .any(|binding| self.used.contains(&binding))
    }

    pub fn brings_only_macros(&self, name: &ImportedName) -> bool {
        name.bindings
            .iter()
            .all(|binding| matches!(self.bindings.get(binding).kind(), BindingKind::Macro(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleUse;
    use crate::core::{BindingId, ImportedBindings, ImportedName};
    use crate::lint::binding_owners::BindingOwners;
    use crate::lint::expanded_source::ExpandedSource;
    use crate::lint::linted_module::LintedModule;
    use crate::source::{SourceId, Span};

    struct UseFixture {
        source: ExpandedSource,
    }

    impl UseFixture {
        fn new(text: &str) -> Self {
            Self {
                source: ExpandedSource::expand(text),
            }
        }

        fn imported(&self, name: &str) -> ImportedName {
            let bindings = &self.source.program().bindings;
            let binding: BindingId = bindings
                .ids()
                .filter(|binding| bindings.get(*binding).name() == name)
                .last()
                .expect("the program binds the name");
            ImportedName {
                span: Span::new(SourceId::new(0), 0..0),
                bindings: ImportedBindings::new(binding),
            }
        }

        fn uses(&mut self, name: &str) -> bool {
            let definitions = self.source.read_back_entry();
            let imported = self.imported(name);
            let program = self.source.program();
            let owners = BindingOwners::of(program);
            let linted = LintedModule::new(program.entry_module(), definitions);
            ModuleUse::of(program, &owners, &linted).refers_to(&imported)
        }

        fn brings_only_macros(&self, name: &str) -> bool {
            let program = self.source.program();
            let owners = BindingOwners::of(program);
            let linted = LintedModule::new(program.entry_module(), Vec::new());
            ModuleUse::of(program, &owners, &linted).brings_only_macros(&self.imported(name))
        }
    }

    #[test]
    fn a_type_named_only_in_a_signature_is_used() {
        let mut fixture = UseFixture::new(
            "(defstruct point :x i64)\n(type origin (fn () point))\n(defn origin () (->point :x 0))",
        );
        assert!(fixture.uses("point"));
    }

    #[test]
    fn a_read_constructor_uses_its_enum() {
        let mut fixture = UseFixture::new(
            "(defenum shape (circle i64) (square i64))\n(defn make () (shape/circle 1))",
        );
        assert!(fixture.uses("shape"));
    }

    #[test]
    fn an_enum_nothing_constructs_is_unused() {
        let mut fixture = UseFixture::new("(defenum shape (circle i64) (square i64))");
        assert!(!fixture.uses("shape"));
    }

    #[test]
    fn a_name_of_a_macro_brings_only_macros() {
        let fixture = UseFixture::new("(defmacro twice (x) x)\n(defn once (x) x)");
        assert!(fixture.brings_only_macros("twice"));
        assert!(!fixture.brings_only_macros("once"));
    }
}
