use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, ModuleId};
use crate::modules::ModuleSources;
use crate::types::{
    DefinitionEntry, DefinitionTable, GenericParam, ProgramReach, Type, TypedExpr, TypedExprKind,
    TypedLeaf, TypedProgram,
};

pub struct Reaches {
    pub main: ProgramReach,
    pub from_user_definitions: FxHashSet<BindingId>,
}

pub struct ReachSearch<'p, 'd> {
    program: &'p TypedProgram,
    definitions: &'d DefinitionTable<'p>,
    reach: ProgramReach,
    type_modules: FxHashMap<BindingId, ModuleId>,
    visited: FxHashSet<BindingId>,
}

impl<'p, 'd> ReachSearch<'p, 'd> {
    pub fn of_main_and_user_definitions(
        program: &'p TypedProgram,
        definitions: &'d DefinitionTable<'p>,
    ) -> Reaches {
        let mut search = Self::from_main(program, definitions);
        let mut main = std::mem::take(&mut search.reach);
        for root in Self::user_definitions(program) {
            search.visit_definition(root);
        }
        main.type_modules = search.type_modules;
        Reaches {
            main,
            from_user_definitions: search.visited,
        }
    }

    fn from_main(program: &'p TypedProgram, definitions: &'d DefinitionTable<'p>) -> Self {
        let mut search = Self {
            program,
            definitions,
            reach: ProgramReach {
                main: Self::find_main(program),
                ..ProgramReach::default()
            },
            type_modules: Self::type_modules(program),
            visited: FxHashSet::default(),
        };
        if let Some(main) = search.reach.main {
            search.visit_definition(main);
        }
        search.reach_implementations();
        search.reach_trait_defaults();
        search
    }

    fn user_definitions(program: &TypedProgram) -> Vec<BindingId> {
        program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions)
            .map(|definition| definition.binding)
            .collect()
    }

    fn find_main(program: &TypedProgram) -> Option<BindingId> {
        program
            .entry_module()
            .definitions
            .iter()
            .find(|definition| {
                program.entry_function == Some(definition.binding)
                    && matches!(definition.value.kind, TypedExprKind::Fn(_))
            })
            .map(|definition| definition.binding)
    }

    fn type_modules(program: &TypedProgram) -> FxHashMap<BindingId, ModuleId> {
        program
            .modules
            .iter()
            .flat_map(|module| {
                module
                    .structs
                    .iter()
                    .chain(module.enums.iter())
                    .map(move |binding| (*binding, module.id))
            })
            .collect()
    }

    fn reach_trait_defaults(&mut self) {
        let traits = &self.program.declarations.traits();
        let defaults: Vec<BindingId> = traits
            .definitions()
            .iter()
            .flat_map(|trait_definition| trait_definition.methods.iter())
            .filter_map(|method| method.default)
            .collect();
        for binding in defaults {
            self.visit_definition(binding);
        }
    }

    fn reach_implementations(&mut self) {
        let traits = &self.program.declarations.traits();
        let implementations: Vec<(Type, ModuleId, Vec<BindingId>)> = traits
            .impls()
            .iter()
            .map(|implementation| {
                (
                    implementation.implementor.clone(),
                    implementation.module,
                    implementation
                        .methods
                        .iter()
                        .map(|method| method.binding)
                        .collect(),
                )
            })
            .collect();
        for (implementor, module, methods) in implementations {
            self.visit_type(&implementor, module);
            for method in methods {
                self.visit_definition(method);
            }
        }
    }

    fn visit_definition(&mut self, binding: BindingId) {
        let Some(DefinitionEntry { definition, module }) = self.definitions.entry(binding) else {
            return;
        };
        if matches!(
            definition.value.kind,
            TypedExprKind::Leaf(TypedLeaf::Rust(_))
        ) || !self.mark_reached(binding)
        {
            return;
        }
        self.visit_type(&definition.scheme.body, module);
        self.visit_params(&definition.scheme.params, module);
        self.visit_expr(&definition.value, module);
    }

    fn mark_reached(&mut self, binding: BindingId) -> bool {
        let first = self.visited.insert(binding);
        if first {
            self.reach.reached.push(binding);
        }
        first
    }

    fn visit_params(&mut self, params: &[GenericParam], module: ModuleId) {
        for param in params {
            let mut arguments: Vec<Type> = param
                .bounds
                .iter()
                .filter_map(|bound| bound.argument.clone())
                .collect();
            arguments.extend(
                self.program
                    .implied_bounds
                    .get(&param.id)
                    .into_iter()
                    .flatten()
                    .filter_map(|bound| bound.argument.clone()),
            );
            for argument in &arguments {
                self.visit_type(argument, module);
            }
        }
    }

    fn visit_expr(&mut self, expr: &TypedExpr, module: ModuleId) {
        self.visit_type(&expr.ty, module);
        if let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = &expr.kind {
            self.reference(*binding, module);
        }
        for binder in expr.binders() {
            self.visit_type(&binder.ty, module);
        }
        for child in expr.children() {
            self.visit_expr(child, module);
        }
    }

    fn visit_type(&mut self, visited: &Type, module: ModuleId) {
        match visited {
            Type::Named(binding, arguments) => {
                arguments
                    .iter()
                    .for_each(|argument| self.visit_type(argument, module));
                self.reach_named(*binding, module);
            }
            Type::Function(_, parameters, result) => {
                parameters
                    .iter()
                    .for_each(|parameter| self.visit_type(parameter, module));
                self.visit_type(result, module);
            }
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.visit_type(inner, module),
            Type::Var(_) | Type::Generic(_) | Type::Primitive(_) | Type::TraitObject(_) => {}
        }
    }

    fn reach_named(&mut self, binding: BindingId, from: ModuleId) {
        let types = &self.program.declarations.types();
        if types.rust_types().get(binding).is_some() {
            self.reach.used_rust_types.insert(binding);
        } else if types.enums().get(binding).is_some() {
            self.reach_enum(binding, from);
        } else {
            self.reach_struct(binding, from);
        }
    }

    fn reach_enum(&mut self, binding: BindingId, from: ModuleId) {
        let Some(owner) = self.type_modules.get(&binding).copied() else {
            return;
        };
        self.note_use(binding, owner, from);
        if !self.mark_reached(binding) {
            return;
        }
        let types = &self.program.declarations.types();
        let payloads: Vec<Type> = types
            .enums()
            .get(binding)
            .map(|definition| definition.payload_types().cloned().collect())
            .unwrap_or_default();
        payloads
            .iter()
            .for_each(|payload| self.visit_type(payload, owner));
    }

    fn reach_struct(&mut self, binding: BindingId, from: ModuleId) {
        let Some(owner) = self.type_modules.get(&binding).copied() else {
            return;
        };
        self.note_use(binding, owner, from);
        if !self.mark_reached(binding) {
            return;
        }
        let types = &self.program.declarations.types();
        let fields: Vec<Type> = types
            .structs()
            .get(binding)
            .map(|definition| {
                definition
                    .fields
                    .iter()
                    .map(|(_, field)| field.clone())
                    .collect()
            })
            .unwrap_or_default();
        fields
            .iter()
            .for_each(|field| self.visit_type(field, owner));
    }

    fn reference(&mut self, binding: BindingId, from: ModuleId) {
        if let Some(entry) = self.definitions.entry(binding) {
            self.note_use(binding, entry.module, from);
        }
        self.visit_definition(binding);
    }

    fn note_use(&mut self, binding: BindingId, owner: ModuleId, from: ModuleId) {
        if owner != from {
            self.reach.used_from_other_modules.insert(binding);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReachSearch, Reaches};
    use crate::core::BindingId;
    use crate::types::{DefinitionTable, TypedProgram, TypedSource};

    struct ReachFixture {
        program: TypedProgram,
    }

    impl ReachFixture {
        const ENTRY_CALLING_ITS_OWN_AND_A_PRELUDE_FUNCTION: &str = "
            (defn helper () 1)
            (defn main () (println (+ (helper) (sum (vec 1 2)))))";

        const PRELUDE_CALL_OUTSIDE_MAIN: &str = "
            (defn total () (sum (vec 1 2)))
            (defn main () (println 1))";

        fn new() -> Self {
            Self::of(Self::ENTRY_CALLING_ITS_OWN_AND_A_PRELUDE_FUNCTION)
        }

        fn of(source: &str) -> Self {
            Self {
                program: TypedSource::expand(source).typed(),
            }
        }

        fn reaches(&self) -> Reaches {
            ReachSearch::of_main_and_user_definitions(
                &self.program,
                &DefinitionTable::of(&self.program),
            )
        }

        fn definition(&self, name: &str) -> BindingId {
            self.program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .map(|definition| definition.binding)
                .find(|binding| self.program.bindings.get(*binding).name() == name)
                .expect("the fixture reaches the definition")
        }
    }

    #[test]
    fn a_definition_used_only_from_its_own_module_is_reached_but_not_shared() {
        let fixture = ReachFixture::new();
        let reach = fixture.reaches().main;
        let helper = fixture.definition("helper");
        assert!(reach.reached.contains(&helper));
        assert!(!reach.used_from_other_modules.contains(&helper));
    }

    #[test]
    fn a_prelude_definition_the_entry_calls_is_used_from_another_module() {
        let fixture = ReachFixture::new();
        let reach = fixture.reaches().main;
        assert!(
            reach
                .used_from_other_modules
                .contains(&fixture.definition("sum"))
        );
    }

    #[test]
    fn a_prelude_definition_called_outside_main_is_reached_only_from_every_user_definition() {
        let fixture = ReachFixture::of(ReachFixture::PRELUDE_CALL_OUTSIDE_MAIN);
        let sum = fixture.definition("sum");
        let reaches = fixture.reaches();
        assert!(!reaches.main.reached.contains(&sum));
        assert!(reaches.from_user_definitions.contains(&sum));
    }

    #[test]
    fn a_user_definition_main_never_calls_is_reached_from_every_user_definition() {
        let fixture = ReachFixture::of(ReachFixture::PRELUDE_CALL_OUTSIDE_MAIN);
        let total = fixture.definition("total");
        let reaches = fixture.reaches();
        assert!(!reaches.main.reached.contains(&total));
        assert!(reaches.from_user_definitions.contains(&total));
    }
}
