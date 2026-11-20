use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::admissible::Admissible;
use super::admissible_types::AdmissibleTypes;
use super::argument_passing::ArgumentPassing;
use super::assumed_collections::AssumedCollections;
use super::callee_borrows::CalleeBorrows;
use super::checked_program::CheckedProgram;
use super::checking_facts::CheckingFacts;
use super::clause_check::ClauseCheck;
use super::collected_rust_trait::CollectedRustTrait;
use super::copy::InferredCopies;
use super::copy_requirement::CopyRequirement;
use super::coverage::{ConstructorCatalog, MatchCoverage, WitnessText};
use super::declarations::Declarations;
use super::declared_generics::DeclaredGenerics;
use super::declared_traits::DeclaredTraits;
use super::declared_types::DeclaredTypes;
use super::deferred_decisions::DeferredDecisions;
use super::definition_outcomes::DefinitionOutcomes;
use super::derivable_traits::DerivableTraits;
use super::derive_blockers::DeriveBlockers;
use super::derive_rules::DeriveRules;
use super::display::TypeDisplay;
use super::enum_item::EnumItem;
use super::explanation_sources::ExplanationSources;
use super::generic_declarations::GenericDeclarations;
use super::generic_registry::GenericRegistry;
use super::graph::{Definition, DefinitionGraph};
use super::impl_bounds::ImplBounds;
use super::impl_item::ImplItem;
use super::implementation_check::ImplementationCheck;
use super::implementation_collector::ImplementationCollector;
use super::implementation_verdict::ImplementationVerdict;
use super::implied_bounds::ImpliedBounds;
use super::index_checks::IndexChecks;
use super::infer::OpenCollections;
use super::instantiation::Instantiation;
use super::literal_settlement::LiteralSettlement;
use super::method_receivers::MethodReceivers;
use super::missing_trait::MissingTrait;
use super::missing_trait_explanation::MissingTraitExplanation;
use super::opaque_positions::OpaquePositions;
use super::parameter_call_search::CalledParameters;
use super::parameter_modes::ParameterModes;
use super::parameter_names::ParameterNames;
use super::passing_facts::PassingFacts;
use super::passing_tables::PassingTables;
use super::peeling::ReferencePeeling;
use super::pinning::LiteralDefaults;
use super::place_types::PlaceTypes;
use super::prelude_types::PreludeTypes;
use super::problems::{MismatchContext, TypeProblems};
use super::reached_traits::{ReachedDeclaration, ReachedTraits};
use super::reopened_parameters::ReopenedParameters;
use super::rust_type_item::RustTypeItem;
use super::selection::Selection;
use super::shared_call::SharedCall;
use super::shared_group::SharedGroup;
use super::shared_member_passing::SharedMemberPassing;
use super::shared_variable_group::SharedVariableGroup;
use super::string_views::StringViews;
use super::template_clauses::{ClauseTarget, TemplateClauses};
use super::trait_declaration_collector::TraitDeclarationCollector;
use super::ty::{Bound, GenericId, GenericParam, Scheme, Type, TypeVar};
use super::type_declaration_collector::TypeDeclarationCollector;
use super::unification::Unification;
use super::unifier::{LiteralKind, Obligation, ObligationSite, Unifier};
use super::user_trait::UserTrait;
use super::variant_payloads::VariantPayloads;
use super::wrapper::FunctionValueWrapper;
use super::{
    LoweredRows, RustTrait, TemplateBorrows, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf,
};
use crate::core::{
    BindingArena, BindingId, BodyItem, CopyTrait, CoreModule, DefinitionForm, Expr, ExprKind, Item,
    ItemKind, KernelForm, Leaf, ModuleId, RustTemplate, StructDerives, StructExpr, TypeExpr,
    TypeExprKind,
};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::expand::{MacroCallLookup, PendingDefinitions};
use crate::modules::ModuleSources;
use crate::source::Span;

#[derive(Clone, Copy)]
pub(super) enum ObligationPass {
    Deferring,
    Final,
}

pub struct TypeChecker {
    pub(super) bindings: BindingArena,
    pub(super) unifier: Unifier,
    pub(super) declarations: Declarations,
    pub(super) generics: GenericRegistry,
    pub(super) schemes: FxHashMap<BindingId, Scheme>,
    pub(super) component: FxHashMap<BindingId, Type>,
    pub(super) locals: FxHashMap<BindingId, Type>,
    pub(super) variant_arm: FxHashMap<BindingId, VariantArmReference>,
    pub(super) unchecked: FxHashSet<BindingId>,
    entry_function: Option<BindingId>,
    pub(super) outcomes: DefinitionOutcomes,
    pub(super) diagnostics: Diagnostics,
    pub(super) warnings: Diagnostics,
    pub(super) missing_annotations: super::missing_annotations::MissingAnnotations,
    pub(super) unnameable_types: Diagnostics,
    pub(super) closure_types: Vec<super::typed::ClosureType>,
    pub(super) type_texts: super::type_text::TypeTexts,
    pub(super) holds_infer_forms: bool,
    pub(super) type_arities: FxHashMap<BindingId, usize>,
    pub(super) written_projections: Vec<super::written_projection::WrittenProjection>,
    pub(super) checked_projections: usize,
    signatures: FxHashMap<BindingId, (TypeExpr, Span)>,
    pub(super) checking_facts: CheckingFacts,
    monomorphic: Vec<Type>,
    pub(super) implied_bounds: ImpliedBounds,
    pub(super) single_field_readers: FxHashMap<BindingId, Rc<str>>,
    pub(super) deferred: DeferredDecisions,
    pub(super) vector_bases: FxHashSet<Span>,
    pub(super) template_borrows: TemplateBorrows,
    template_clauses: TemplateClauses,
    pub(super) recur_target: Option<super::infer::RecurTarget>,
    pub(super) checking_function_body: bool,
    pub(super) try_target: Option<super::infer::TryTarget>,
    pub(super) changed_structs: FxHashSet<BindingId>,
    pub(super) inferred_copies: InferredCopies,
    call_origins: Rc<dyn MacroCallLookup>,
    macro_field_reads: FxHashMap<Span, Rc<str>>,
    copy_trait: Option<BindingId>,
    pub(super) structural_uses: super::structural_uses::StructuralUses,
    pub(super) reported_path_clashes: FxHashSet<super::structural_uses::StructuralUse>,
    impl_methods: FxHashSet<BindingId>,
    pub(super) synthetic_bindings: FxHashSet<BindingId>,
    pub(super) pinned: FxHashMap<BindingId, Type>,
    pub(super) literal_defaults: LiteralDefaults,
    pub(super) assumed_collections: AssumedCollections,
    default_literal_parameters: bool,
    pub(super) recursive_group: Vec<BindingId>,
    pub(super) reported_unknown_try_groups: FxHashSet<BindingId>,
    pub(super) string_view: Option<BindingId>,
    pub(super) checking: super::typed::CheckWork,
    pub(super) inferring: super::inferred_span::InferredSpan,
}

#[derive(Clone, Copy)]
pub(super) struct VariantArmReference {
    pub outer: Option<BindingId>,
    pub variant: BindingId,
}

impl TypeChecker {
    pub(super) fn new(bindings: BindingArena) -> Self {
        Self {
            bindings,
            unifier: Unifier::default(),
            declarations: Declarations::default(),
            generics: GenericRegistry::default(),
            schemes: FxHashMap::default(),
            component: FxHashMap::default(),
            locals: FxHashMap::default(),
            variant_arm: FxHashMap::default(),
            unchecked: FxHashSet::default(),
            entry_function: None,
            outcomes: DefinitionOutcomes::default(),
            diagnostics: Diagnostics::default(),
            warnings: Diagnostics::default(),
            missing_annotations: super::missing_annotations::MissingAnnotations::default(),
            unnameable_types: Diagnostics::default(),
            closure_types: Vec::new(),
            type_texts: super::type_text::TypeTexts::default(),
            holds_infer_forms: false,
            type_arities: FxHashMap::default(),
            written_projections: Vec::new(),
            checked_projections: 0,
            signatures: FxHashMap::default(),
            checking_facts: CheckingFacts::default(),
            monomorphic: Vec::new(),
            implied_bounds: ImpliedBounds::default(),
            single_field_readers: FxHashMap::default(),
            deferred: DeferredDecisions::default(),
            vector_bases: FxHashSet::default(),
            template_borrows: TemplateBorrows::default(),
            template_clauses: TemplateClauses::default(),
            recur_target: None,
            checking_function_body: false,
            try_target: None,
            changed_structs: FxHashSet::default(),
            inferred_copies: InferredCopies::default(),
            call_origins: <dyn MacroCallLookup>::empty(),
            macro_field_reads: FxHashMap::default(),
            copy_trait: None,
            structural_uses: super::structural_uses::StructuralUses::default(),
            reported_path_clashes: FxHashSet::default(),
            impl_methods: FxHashSet::default(),
            synthetic_bindings: FxHashSet::default(),
            pinned: FxHashMap::default(),
            literal_defaults: LiteralDefaults::default(),
            assumed_collections: AssumedCollections::default(),
            default_literal_parameters: false,
            recursive_group: Vec::new(),
            reported_unknown_try_groups: FxHashSet::default(),
            string_view: None,
            checking: super::typed::CheckWork::default(),
            inferring: super::inferred_span::InferredSpan::default(),
        }
    }

    fn collect_declarations(&mut self, modules: &[CoreModule], graph: &DefinitionGraph) {
        self.record_type_arities(modules);
        self.record_projections(modules);
        let reached = graph.reach().traits();
        self.record_trait_arguments(modules, reached);
        for module in modules {
            for item in &module.items {
                let collected = if let ItemKind::RustTrait(declaration) = &item.kind {
                    self.collect_rust_trait(declaration, reached)
                } else {
                    self.collect_item(module.id, item)
                };
                if let Err(diagnostic) = collected {
                    self.diagnostics.push(diagnostic);
                }
            }
        }
        self.share_projections();
        self.check_written_projections();
        self.declarations.builder().types().plan_boxes();
        self.collect_trait_impls(modules);
        self.record_structural_traits(modules);
        self.register_derived_traits(modules);
        self.report_supertraits();
        self.check_trait_claims(modules, reached);
        self.copy_trait = self
            .declarations
            .traits()
            .table
            .with_paths(&[CopyTrait::PATH])
            .first()
            .copied();
        self.record_definition_facts(graph.definitions());
        let written = graph.reach().written_bodies().clone();
        self.declarations.builder().record_written_bodies(written);
        self.declarations.freeze();
    }

    fn record_prelude_definitions(&mut self, modules: &[CoreModule], saved: &PendingDefinitions) {
        let defined = modules
            .iter()
            .filter(|module| ModuleSources::is_prelude(&module.path))
            .flat_map(CoreModule::definitions)
            .map(|(binding, _)| binding);
        self.checking_facts.prelude_definitions = defined.chain(saved.saved_bindings()).collect();
    }

    fn record_definition_facts(&mut self, definitions: &[Definition]) {
        for definition in definitions {
            self.record_single_field_reader(definition.binding, definition.value);
            self.record_written_parameters(definition.binding, definition.value);
            self.assumed_collections
                .record_parameters(definition.binding, definition.value);
        }
    }

    fn record_written_parameters(&mut self, binding: BindingId, value: &Expr) {
        let ExprKind::Fn(lambda) = value.kind() else {
            return;
        };
        let signature = self
            .signatures
            .get(&binding)
            .map(|(signature, _)| signature);
        let written = lambda
            .params
            .iter()
            .enumerate()
            .map(|(position, param)| {
                param.annotation.is_some()
                    || signature.is_some_and(|signature| {
                        Self::signature_parameter_is_written(signature, position)
                    })
            })
            .collect();
        self.checking_facts
            .written_parameters
            .insert(binding, written);
    }

    fn signature_parameter_is_written(signature: &TypeExpr, position: usize) -> bool {
        match signature.kind() {
            TypeExprKind::Function(_, parameters, _) => parameters
                .get(position)
                .is_none_or(|parameter| !matches!(parameter.kind(), TypeExprKind::Hole)),
            _ => true,
        }
    }

    fn collect_item(&mut self, module: ModuleId, item: &Item) -> Result<(), Diagnostic> {
        match &item.kind {
            ItemKind::Def(..) | ItemKind::Signature(..) | ItemKind::Derive(..) => {
                self.collect_value_item(item)
            }
            ItemKind::MismatchHelp(help) => {
                let row = self
                    .type_collector()
                    .collect_mismatch_help(help, item.span)?;
                self.declarations.builder().types().add_mismatch_help(row)
            }
            _ => self.collect_type_item(module, item),
        }
    }

    fn collect_value_item(&mut self, item: &Item) -> Result<(), Diagnostic> {
        if let ItemKind::Signature(binding, signature) = &item.kind {
            self.signatures
                .insert(*binding, (signature.clone(), item.span));
            return Ok(());
        }
        Ok(())
    }

    fn collect_type_item(&mut self, module: ModuleId, item: &Item) -> Result<(), Diagnostic> {
        let kind = &item.kind;
        if let ItemKind::Struct(binding, params, fields) = kind {
            let collected = self.type_collector().collect_struct(params, fields)?;
            let types = self.declarations.builder().types();
            types.enter_struct(*binding, collected, &mut self.diagnostics);
            return Ok(());
        }
        if let ItemKind::Enum(binding, binders, variants, rust, definer) = kind {
            let collected = self.type_collector().collect_enum(EnumItem {
                binding: *binding,
                binders,
                variants,
                rust,
                definer,
            })?;
            let types = self.declarations.builder().types();
            types.enter_enum(*binding, collected, &mut self.diagnostics);
            return Ok(());
        }
        if let ItemKind::RustType(binding, binders, unsized_parameters, template, clauses) = kind {
            let rust_type = self.type_collector().collect_rust_type(RustTypeItem {
                binders,
                unsized_parameters,
                template,
                clauses,
            })?;
            self.declarations
                .builder()
                .types()
                .add_rust_type(*binding, rust_type);
            return Ok(());
        }
        if let ItemKind::Trait(
            binding,
            rust_name,
            self_variable,
            supertraits,
            associated,
            methods,
        ) = kind
        {
            return self.collect_user_trait(UserTrait {
                module,
                binding: *binding,
                rust_name,
                self_variable: *self_variable,
                supertraits,
                associated,
                methods,
            });
        }
        Ok(())
    }

    fn record_trait_arguments(&mut self, modules: &[CoreModule], reached: &ReachedTraits) {
        for ReachedDeclaration { declaration, .. } in reached.declarations(modules) {
            if (declaration.implementors.get().iter())
                .any(|entry| Self::entry_uses_with(&entry.implementor))
            {
                self.checking_facts
                    .trait_arguments
                    .insert(declaration.binding);
            }
        }
    }

    fn entry_uses_with(entry: &TypeExpr) -> bool {
        match entry.kind() {
            TypeExprKind::With(..) => true,
            TypeExprKind::Where(_, inner) => Self::entry_uses_with(inner),
            _ => false,
        }
    }

    fn check_trait_claims(&mut self, modules: &[CoreModule], reached: &ReachedTraits) {
        for ReachedDeclaration { span, declaration } in reached.declarations(modules) {
            let trait_binding = &declaration.binding;
            let path = &declaration.path;
            for implementor in declaration.implementors.get() {
                let implementor = Self::implemented_type(&implementor.implementor);
                let TypeExprKind::Named(binding, _) = implementor.kind() else {
                    continue;
                };
                let Some(definition) = self.declarations.types().structs.get(*binding) else {
                    continue;
                };
                if definition
                    .derives
                    .iter()
                    .any(|derived| *derived == StructDerives::derive_name(path))
                {
                    continue;
                }
                let struct_name = self.bindings.get(*binding).name().to_string();
                let trait_name = self.bindings.get(*trait_binding).name().to_string();
                let definer = self.macro_name(span);
                self.diagnostics.push(TypeProblems::trait_claim_on_struct(
                    definer
                        .as_deref()
                        .unwrap_or(KernelForm::Definition(DefinitionForm::RustTrait).name()),
                    &struct_name,
                    &trait_name,
                    implementor.span(),
                ));
            }
        }
    }

    fn implemented_type(implementor: &TypeExpr) -> &TypeExpr {
        match implementor.kind() {
            TypeExprKind::With(body, _) => body,
            TypeExprKind::Where(_, inner) => Self::implemented_type(inner),
            _ => implementor,
        }
    }

    fn record_single_field_reader(&mut self, binding: BindingId, value: &Expr) {
        let ExprKind::Fn(lambda) = value.kind() else {
            return;
        };
        let [parameter] = lambda.params.as_slice() else {
            return;
        };
        let [BodyItem::Expr(body)] = lambda.body.items.as_slice() else {
            return;
        };
        let ExprKind::Struct(StructExpr::Field(owner, label)) = body.kind() else {
            return;
        };
        let ExprKind::Leaf(Leaf::Var(owner_binding)) = owner.kind() else {
            return;
        };
        if *owner_binding == parameter.binding {
            self.single_field_readers
                .insert(binding, label.name.clone());
        }
    }

    fn record_type_arities(&mut self, modules: &[CoreModule]) {
        for item in modules.iter().flat_map(|module| &module.items) {
            if let ItemKind::Struct(binding, params, _)
            | ItemKind::Enum(binding, params, ..)
            | ItemKind::RustType(binding, params, ..) = &item.kind
            {
                let arity = params.iter().filter(|binder| !binder.lifetime).count();
                self.type_arities.insert(*binding, arity);
            }
        }
    }

    fn check_definitions(
        &mut self,
        entry_function: Option<BindingId>,
        graph: &DefinitionGraph,
    ) -> FxHashMap<BindingId, TypedDefinition> {
        let dependencies = graph.dependencies();
        self.unchecked = dependencies.unchecked(entry_function.as_slice());
        self.entry_function = entry_function;
        let by_binding: FxHashMap<BindingId, &Definition> = graph
            .definitions()
            .iter()
            .map(|definition| (definition.binding, definition))
            .collect();
        let mut typed = FxHashMap::default();
        for component in graph.components() {
            let members: Vec<&Definition> = component
                .iter()
                .filter(|binding| !self.unchecked.contains(binding))
                .map(|binding| by_binding[binding])
                .collect();
            let bindings: Vec<BindingId> = members.iter().map(|member| member.binding).collect();
            if self.outcomes.fail_when_blocked(dependencies, &bindings) {
                continue;
            }
            let reported_before = self.diagnostics.len();
            typed.extend(self.check_component(&members));
            let reported_errors = self.diagnostics.len() - reported_before;
            self.outcomes
                .finish_group(dependencies, &bindings, reported_errors);
        }
        typed
    }

    pub(super) fn admissible_types(&self) -> AdmissibleTypes<'_> {
        AdmissibleTypes::new(self.generics.params(), &self.declarations.traits().table)
    }

    pub(super) fn copy_rules(&self) -> super::copy::CopyRules<'_> {
        self.passing_tables().copy_rules()
    }

    pub(super) fn variant_payloads(&self) -> VariantPayloads<'_> {
        VariantPayloads::new(self.declarations.types())
    }

    pub(super) fn declared_types(&self) -> DeclaredTypes<'_> {
        self.passing_tables().declared_types()
    }

    pub(super) fn declared_traits(&self) -> DeclaredTraits<'_> {
        DeclaredTraits::new(self.declarations.traits())
    }

    pub(super) fn derivability(&self) -> DerivableTraits<'_> {
        DerivableTraits::new(self.declarations.traits())
    }

    pub(super) fn derive_rules(&self) -> DeriveRules<'_> {
        let blockers = DeriveBlockers::new(&self.declarations, self.variant_payloads());
        DeriveRules::new(&self.declarations, blockers, &self.changed_structs)
    }

    fn check_opaque_positions(
        &mut self,
        modules: &[CoreModule],
        definitions: &FxHashMap<BindingId, TypedDefinition>,
    ) {
        let unnameable = self.opaque_positions(modules, definitions).unnameable();
        let display = self.display();
        let diagnostics: Diagnostics = unnameable
            .iter()
            .map(|unnameable| unnameable.diagnostic(&display))
            .collect();
        self.unnameable_types.extend(diagnostics);
    }

    fn opaque_positions<'c>(
        &'c self,
        modules: &'c [CoreModule],
        definitions: &'c FxHashMap<BindingId, TypedDefinition>,
    ) -> OpaquePositions<'c> {
        let checked = CheckedProgram {
            modules,
            definitions,
            closures: &self.closure_types,
        };
        let receivers = MethodReceivers::new(&self.template_borrows);
        OpaquePositions::new(self.declarations.types(), receivers, checked)
    }

    pub(super) fn reference_peeling(&self) -> ReferencePeeling<'_> {
        ReferencePeeling::new(&self.unifier)
    }

    fn missing_trait_explanation(&self) -> MissingTraitExplanation<'_> {
        MissingTraitExplanation::new(ExplanationSources {
            declarations: &self.declarations,
            bindings: &self.bindings,
            display: self.display(),
            derive_rules: self.derive_rules(),
        })
    }

    fn literal_settlement(&self) -> LiteralSettlement<'_> {
        LiteralSettlement::new(&self.unifier, &self.declarations.traits().table)
    }

    fn clause_check(&mut self) -> ClauseCheck<'_> {
        let open = self.unifier.fresh();
        ClauseCheck::new(&self.declarations.traits().table, self.display(), open)
    }

    pub(super) fn index_checks(&self) -> IndexChecks<'_> {
        IndexChecks::new(&self.template_borrows, self.declarations.types())
    }

    pub(super) fn prelude_types(&self) -> PreludeTypes<'_> {
        PreludeTypes::new(&self.declarations.types().rust_types)
    }

    pub(super) fn function_value_wrapper(&mut self) -> FunctionValueWrapper<'_> {
        FunctionValueWrapper::new(&mut self.bindings)
    }

    pub(super) fn place_types(&self) -> PlaceTypes<'_> {
        PlaceTypes::new(&self.unifier, self.declared_types())
    }

    pub(super) fn string_views(&self) -> StringViews<'_> {
        StringViews::new(&self.unifier, self.place_types(), self.string_view)
    }

    pub(super) fn shared_variable_group<'g>(
        &'g self,
        group: &'g SharedGroup,
        callee: Option<&'g TypedExpr>,
    ) -> SharedVariableGroup<'g> {
        let callee = CalleeBorrows::new(callee, &self.template_borrows);
        let call = SharedCall { group, callee };
        SharedVariableGroup::new(call, self.passing_tables(), &self.unifier)
    }

    pub(super) fn shared_member_passing<'c>(
        &'c self,
        callee: Option<&'c TypedExpr>,
    ) -> SharedMemberPassing<'c> {
        let callee = CalleeBorrows::new(callee, &self.template_borrows);
        SharedMemberPassing::new(self.place_types(), &self.unifier, callee)
    }

    pub(super) fn unification(&mut self) -> Unification<'_> {
        let (tables, unifier, _) = self.passing_borrows();
        tables.unification(unifier)
    }

    pub(super) fn called_parameters<'m>(
        &'m self,
        modules: &'m [CoreModule],
    ) -> CalledParameters<'m> {
        CalledParameters::new(modules, &self.bindings)
    }

    pub(super) fn match_coverage(&self) -> MatchCoverage<'_> {
        let catalog = ConstructorCatalog::new(
            self.reference_peeling(),
            self.declarations.types(),
            self.variant_payloads(),
        );
        let text = WitnessText::new(&self.bindings, &self.declarations.types().enums);
        MatchCoverage::new(&self.bindings, catalog, text)
    }

    pub(super) fn require_copy(&mut self, value: &Type) {
        CopyRequirement::new(self.copy_trait, &mut self.unifier).require(value, Vec::new());
    }

    pub(super) fn argument_passing(&mut self) -> ArgumentPassing<'_> {
        let (tables, unifier, deferred) = self.passing_borrows();
        ArgumentPassing::new(tables, unifier, deferred)
    }

    fn reopened_parameters(&mut self) -> ReopenedParameters<'_> {
        let (tables, unifier, deferred) = self.passing_borrows();
        ReopenedParameters::new(tables, unifier, deferred)
    }

    fn passing_borrows(&mut self) -> (PassingTables<'_>, &mut Unifier, &mut DeferredDecisions) {
        let tables = PassingTables {
            declarations: &self.declarations,
            bindings: &self.bindings,
            generics: self.generics.params(),
            single_field_readers: &self.single_field_readers,
            copy_trait: self.copy_trait,
            enclosing: self.inferring.innermost(),
        };
        (tables, &mut self.unifier, &mut self.deferred)
    }

    pub(super) fn passing_facts(&self) -> PassingFacts<'_> {
        PassingFacts::new(self.passing_tables(), &self.unifier)
    }

    fn passing_tables(&self) -> PassingTables<'_> {
        PassingTables {
            declarations: &self.declarations,
            bindings: &self.bindings,
            generics: self.generics.params(),
            single_field_readers: &self.single_field_readers,
            copy_trait: self.copy_trait,
            enclosing: self.inferring.innermost(),
        }
    }

    pub(super) fn display(&self) -> TypeDisplay<'_> {
        self.passing_tables().display()
    }

    fn resolve_missing_annotations(&mut self) {
        if !self.diagnostics.has_errors() {
            let missing = std::mem::take(&mut self.missing_annotations);
            let reports = missing.one_per_variable(|var| self.open_roots(var));
            self.diagnostics.extend(reports);
        }
    }

    fn open_roots(&self, var: TypeVar) -> Vec<TypeVar> {
        let roots = self.free_vars(&Type::Var(var));
        if roots.is_empty() { vec![var] } else { roots }
    }

    pub(super) fn record_macro_field_read(&mut self, label: &crate::core::FieldLabel, span: Span) {
        if self.call_origins.introducing_macro(label.span).is_some() {
            return;
        }
        if let Some(name) = self.macro_name(span) {
            self.macro_field_reads.insert(span, name);
        }
    }

    pub(super) fn macro_name(&self, span: Span) -> Option<Rc<str>> {
        Some(self.call_origins.introducing_macro(span)?.name)
    }

    pub(super) fn generic_declarations(&mut self) -> GenericDeclarations<'_> {
        GenericDeclarations::new(&mut self.generics, &mut self.unifier)
    }

    fn impl_bounds(&mut self) -> ImplBounds<'_> {
        let generics = GenericDeclarations::new(&mut self.generics, &mut self.unifier);
        ImplBounds::new(&mut self.declarations, &self.implied_bounds, generics)
    }

    fn written_type_parts(
        &mut self,
    ) -> (
        super::written_type_lowering::WrittenTypeLowering<'_>,
        super::written_type_facts::WrittenTypeFacts<'_>,
    ) {
        let facts = super::written_type_facts::WrittenTypeFacts {
            bindings: &self.bindings,
            declarations: &self.declarations,
            checking_facts: &self.checking_facts,
            type_arities: &self.type_arities,
        };
        let generics = GenericDeclarations::new(&mut self.generics, &mut self.unifier);
        let records = super::written_type_records::WrittenTypeRecords {
            projections: &mut self.written_projections,
            diagnostics: &mut self.diagnostics,
        };
        let lowering =
            super::written_type_lowering::WrittenTypeLowering::new(facts, generics, records);
        (lowering, facts)
    }

    pub(super) fn written_types(
        &mut self,
    ) -> super::written_type_lowering::WrittenTypeLowering<'_> {
        self.written_type_parts().0
    }

    pub(super) fn type_collector(&mut self) -> TypeDeclarationCollector<'_> {
        let (lowering, facts) = self.written_type_parts();
        TypeDeclarationCollector::new(lowering, facts.bindings, facts.declarations.types())
    }

    fn trait_collector(&mut self) -> TraitDeclarationCollector<'_> {
        TraitDeclarationCollector::new(self.written_types())
    }

    pub(super) fn written_schemes(&mut self) -> super::scheme_lowering::SchemeLowering<'_> {
        let (lowering, facts) = self.written_type_parts();
        let variables = super::written_type_variables::WrittenTypeVariables::new(facts.bindings);
        super::scheme_lowering::SchemeLowering::new(lowering, variables)
    }

    pub(super) fn instantiation(&mut self) -> Instantiation<'_> {
        Instantiation::new(&self.generics, &self.implied_bounds, &mut self.unifier)
    }

    fn discharge_obligations(&mut self, pass: ObligationPass) {
        self.recheck_deferred_equalities();
        for obligation in self.unifier.take_obligations() {
            self.with_operand_mark(obligation.inside_operand, |checker| {
                checker.discharge(obligation, pass)
            });
        }
    }

    pub(super) fn with_operand_mark<T>(
        &mut self,
        inside_operand: bool,
        work: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let outer = self.unifier.inside_operand();
        self.unifier.set_inside_operand(inside_operand);
        let result = work(self);
        self.unifier.set_inside_operand(outer);
        result
    }

    fn discharge(&mut self, obligation: Obligation, pass: ObligationPass) {
        let bound = self.resolve_bound(&obligation.bound);
        let implementor = self.unifier.resolve(&obligation.implementor);
        match implementor {
            Type::Var(var) => self.unifier.add_bound(var, bound),
            Type::Generic(generic) => {
                self.check_generic_bound(generic, bound, obligation.site.span)
            }
            _ => self.select_implementor(bound, &implementor, obligation.site, pass),
        }
    }

    fn resolve_bound(&self, bound: &Bound) -> Bound {
        Bound {
            binding: bound.binding,
            argument: bound
                .argument
                .as_ref()
                .map(|argument| self.unifier.resolve(argument)),
            origins: bound.origins.clone(),
        }
    }

    fn select_implementor(
        &mut self,
        bound: Bound,
        implementor: &Type,
        site: ObligationSite,
        pass: ObligationPass,
    ) {
        if self.declarations.types().boxed_object(implementor) == Some(bound.binding) {
            return;
        }
        match self.literal_settlement().select(&bound, implementor) {
            Selection::Implemented {
                requirements,
                argument,
            } => self.adopt_selection(bound, implementor, requirements, argument, site, pass),
            Selection::Missing if !implementor.has_vars() => {
                self.report_missing_trait(&bound, implementor, site.span)
            }
            Selection::Undecided | Selection::Missing => self.leave_unresolved(
                Obligation {
                    bound,
                    implementor: implementor.clone(),
                    site,
                    inside_operand: self.unifier.inside_operand(),
                },
                pass,
            ),
        }
    }

    fn adopt_selection(
        &mut self,
        bound: Bound,
        implementor: &Type,
        requirements: Vec<super::selection::Requirement>,
        argument: Option<Type>,
        site: ObligationSite,
        pass: ObligationPass,
    ) {
        if let (Some(argument), Some(expected)) = (argument, &bound.argument)
            && self.unifier.unify(expected, &argument, site.span).is_err()
        {
            self.report_missing_trait(&bound, implementor, site.span);
            return;
        }
        self.record_structural_use(&bound, implementor, site.span);
        for requirement in requirements {
            let required = Obligation {
                bound: requirement.bound,
                implementor: requirement.implementor,
                site,
                inside_operand: self.unifier.inside_operand(),
            };
            self.discharge(required, pass);
        }
    }

    pub(super) fn defaults_literal_parameters(&self) -> bool {
        self.default_literal_parameters
    }

    fn leave_unresolved(&mut self, obligation: Obligation, pass: ObligationPass) {
        match pass {
            ObligationPass::Deferring => self.unifier.defer(obligation),
            ObligationPass::Final if obligation.inside_operand => {}
            ObligationPass::Final => self.report_open_obligation(&obligation),
        }
    }

    fn report_open_obligation(&mut self, obligation: &Obligation) {
        let mut open_vars = self.free_vars(&obligation.implementor);
        if let Some(argument) = &obligation.bound.argument {
            open_vars.extend(self.free_vars(argument));
        }
        let site = obligation.site;
        let label_span = site.bound_site.unwrap_or(site.span);
        self.missing_annotations
            .defer(open_vars, TypeProblems::cannot_infer_bound(label_span));
    }

    fn check_component(&mut self, members: &[&Definition]) -> Vec<(BindingId, TypedDefinition)> {
        let inferred: Vec<&Definition> = members
            .iter()
            .copied()
            .filter(|member| !self.assign_rust_scheme(member))
            .collect();
        let signatures: FxHashMap<BindingId, Scheme> = inferred
            .iter()
            .filter_map(|member| self.prepare_member(member))
            .collect();
        self.recursive_group = members.iter().map(|member| member.binding).collect();
        let mut typed: Vec<(BindingId, TypedExpr)> = Vec::new();
        for member in &inferred {
            match self.infer_member(member) {
                Ok(value) => typed.push((member.binding, value)),
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    self.outcomes.fail([member.binding]);
                    let failed = self.generic_declarations().failed_scheme();
                    self.schemes.insert(member.binding, failed);
                }
            }
        }
        self.close_open_set_ats();
        let decided = self.decide_pending_borrows();
        self.reopened_parameters().close();
        for (binding, value) in &mut typed {
            decided.patch(value);
            self.borrow_unsized_result(*binding, value);
        }
        self.discharge_obligations(ObligationPass::Deferring);
        let generics: Vec<(BindingId, &TypedExpr)> = typed
            .iter()
            .map(|(binding, value)| (*binding, value))
            .collect();
        if !generics.is_empty() {
            let signature_count = signatures
                .values()
                .map(|signature| signature.params.len())
                .max()
                .unwrap_or_default();
            self.generalize_component(&generics, signature_count);
        }
        let mut results = Vec::new();
        for (binding, value) in typed {
            let signature = signatures.get(&binding);
            let written_result = signature.is_some_and(Self::writes_function_result);
            let scheme = self.finish_scheme(binding, &value, signature);
            self.schemes.insert(binding, scheme.clone());
            let span = members
                .iter()
                .find(|member| member.binding == binding)
                .map_or(value.span, |member| member.span);
            results.push((
                binding,
                TypedDefinition {
                    binding,
                    scheme,
                    value,
                    span,
                    written_result,
                },
            ));
        }
        results.extend(
            members
                .iter()
                .filter_map(|member| self.rust_definition(member)),
        );
        self.component.clear();
        self.recursive_group.clear();
        results
    }

    fn writes_function_result(signature: &Scheme) -> bool {
        matches!(
            &signature.body,
            crate::types::Type::Function(_, _, result)
                if matches!(
                    result.as_ref(),
                    crate::types::Type::Function(kind, _, _) if *kind != crate::core::ClosureKind::Fn
                )
        )
    }

    fn borrow_unsized_result(&mut self, binding: BindingId, value: &mut TypedExpr) {
        let Type::Function(kind, parameters, result) = self.unifier.resolve(&value.ty) else {
            return;
        };
        if !result.is_unsized() {
            return;
        }
        let borrowed = Type::Function(kind, parameters, Box::new(Type::shared(*result)));
        value.ty = borrowed.clone();
        self.component.insert(binding, borrowed);
    }

    fn assign_rust_scheme(&mut self, member: &Definition) -> bool {
        let ExprKind::Leaf(Leaf::Rust(template)) = member.value.kind() else {
            return false;
        };
        let scheme = self.written_schemes().lower_scheme(&template.type_expr);
        let scheme = scheme.unwrap_or_else(|diagnostic| {
            self.diagnostics.push(diagnostic);
            self.generic_declarations().failed_scheme()
        });
        self.collect_clauses(member.binding, template, &scheme);
        let name = super::TemplateName::Declared(member.binding, &template.template);
        let rust_types = &self.declarations.types().rust_types;
        self.template_borrows.register(name, &scheme, rust_types);
        self.schemes.insert(member.binding, scheme);
        true
    }

    fn collect_clauses(&mut self, binding: BindingId, template: &RustTemplate, scheme: &Scheme) {
        let lowered = template
            .clauses
            .iter()
            .map(|clause| {
                let target = self.clause_target(&clause.target)?;
                Ok((target, Rc::clone(&clause.template)))
            })
            .collect::<Result<Vec<_>, Diagnostic>>();
        match lowered {
            Ok(clauses) => {
                for problem in self.clause_check().problems(scheme, template, &clauses) {
                    self.diagnostics.push(problem);
                }
                self.template_clauses.add(binding, clauses);
            }
            Err(diagnostic) => self.diagnostics.push(diagnostic),
        }
    }

    fn clause_target(&mut self, target: &TypeExpr) -> Result<ClauseTarget, Diagnostic> {
        match target.kind() {
            TypeExprKind::Trait(binding) => Ok(ClauseTarget::Trait(Bound::plain(*binding))),
            _ => self
                .written_types()
                .lower_without_generics(target)
                .map(ClauseTarget::Type),
        }
    }

    fn rust_definition(&self, member: &Definition) -> Option<(BindingId, TypedDefinition)> {
        let ExprKind::Leaf(Leaf::Rust(template)) = member.value.kind() else {
            return None;
        };
        let scheme = self.schemes.get(&member.binding)?.clone();
        let value = TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Rust(template.clone())),
            ty: scheme.body.clone(),
            span: member.value.span(),
        };
        Some((
            member.binding,
            TypedDefinition {
                binding: member.binding,
                scheme,
                value,
                span: member.span,
                written_result: false,
            },
        ))
    }

    fn prepare_member(&mut self, member: &Definition) -> Option<(BindingId, Scheme)> {
        if let Some((signature, _)) = self.signatures.get(&member.binding).cloned() {
            return match self.written_schemes().lower_scheme(&signature) {
                Ok(scheme) => {
                    if self.impl_methods.contains(&member.binding) {
                        self.implied_bounds.mark_impl_generics(&scheme.params);
                    }
                    self.component.insert(member.binding, scheme.body.clone());
                    Some((member.binding, scheme))
                }
                Err(diagnostic) => {
                    self.diagnostics.push(diagnostic);
                    let fresh = self.unifier.fresh();
                    self.component.insert(member.binding, fresh);
                    None
                }
            };
        }
        if let Some(pinned) = self.pinned.get(&member.binding).cloned() {
            let pinned = self.open_unpinned_parameters(&pinned);
            let reopened =
                (self.reopened_parameters()).reopen_borrowed_copy_parameters(pinned, member.span);
            self.component.insert(member.binding, reopened);
            return None;
        }
        let shape = self.unwritten_shape(member.value);
        self.component.insert(member.binding, shape);
        None
    }

    fn open_unpinned_parameters(&mut self, pinned: &Type) -> Type {
        let mut unpinned = Vec::new();
        Self::collect_generics(pinned, &mut unpinned);
        let fresh = unpinned
            .into_iter()
            .map(|generic| (generic, self.unifier.fresh()))
            .collect();
        pinned.substitute(&fresh)
    }

    fn unwritten_shape(&mut self, value: &Expr) -> Type {
        let ExprKind::Fn(lambda) = value.kind() else {
            return self.unifier.fresh();
        };
        let parameters = lambda.params.iter().map(|_| self.unifier.fresh()).collect();
        Type::Function(
            crate::core::ClosureKind::Fn,
            parameters,
            Box::new(self.unifier.fresh()),
        )
    }

    fn infer_member(&mut self, member: &Definition) -> Result<TypedExpr, Diagnostic> {
        self.locals.clear();
        self.variant_arm.clear();
        let expected = self.component[&member.binding].clone();
        let context = match self.signatures.get(&member.binding) {
            Some((_, signature)) => MismatchContext::Signature {
                signature: *signature,
            },
            None => MismatchContext::Plain,
        };
        self.infer_against(member.value, &expected, context)
    }

    fn finish_scheme(
        &mut self,
        binding: BindingId,
        value: &TypedExpr,
        signature: Option<&Scheme>,
    ) -> Scheme {
        let inferred = self.unifier.resolve(&self.component[&binding]);
        if let Some(signature) = signature {
            return Scheme {
                lifetimes: signature.lifetimes.clone(),
                params: signature.params.clone(),
                body: inferred,
            };
        }
        if matches!(value.kind, TypedExprKind::Fn(_)) {
            let mut params = self.mentioned_generics(&inferred);
            for param in &mut params {
                for bound in &mut param.bounds {
                    bound.argument = bound
                        .argument
                        .as_ref()
                        .map(|argument| self.unifier.resolve(argument));
                }
            }
            return Scheme {
                lifetimes: Vec::new(),
                params,
                body: inferred,
            };
        }
        self.monomorphic.push(inferred.clone());
        Scheme::monomorphic(inferred)
    }

    fn generalize_component(&mut self, members: &[(BindingId, &TypedExpr)], start: usize) {
        let vars = self.component_vars(members);
        let span = members
            .first()
            .map(|(_, value)| value.span)
            .expect("a recursive group has at least one member");
        while self.link_round(&vars, span) {}
        self.settle_entry_success(members, span);
        let pinnable_vars = self.pinnable_vars(members);
        self.generalize_vars(vars, start, span, &pinnable_vars);
        self.discharge_obligations(ObligationPass::Deferring);
    }

    fn settle_entry_success(&mut self, members: &[(BindingId, &TypedExpr)], span: Span) {
        let entry = members
            .iter()
            .find(|(binding, _)| Some(*binding) == self.entry_function);
        if let Some(success) = entry.and_then(|(_, value)| self.open_result_success(&value.ty)) {
            self.unifier.unify(&success, &Type::unit(), span).ok();
        }
    }

    fn open_result_success(&self, function: &Type) -> Option<Type> {
        let Type::Function(_, _, result) = self.unifier.resolve(function) else {
            return None;
        };
        let Type::Named(binding, arguments) = *result else {
            return None;
        };
        let [success, _] = arguments.as_slice() else {
            return None;
        };
        let success = self.unifier.shallow(success);
        let open = matches!(success, Type::Var(var) if self.unifier.bounds(var).is_empty());
        (open
            && self
                .declarations
                .types()
                .enums
                .get(binding)?
                .is_result_shaped())
        .then_some(success)
    }

    fn pinnable_vars(&self, members: &[(BindingId, &TypedExpr)]) -> FxHashSet<TypeVar> {
        let mut pinnable = self.parameter_vars(members);
        pinnable.extend(self.result_vars(members));
        pinnable
    }

    fn parameter_vars(&self, members: &[(BindingId, &TypedExpr)]) -> FxHashSet<TypeVar> {
        Self::parameter_types(members)
            .iter()
            .flat_map(|parameter| self.pinnable_parameter_vars(parameter))
            .collect()
    }

    fn pinnable_parameter_vars(&self, parameter: &Type) -> Vec<TypeVar> {
        match self.unifier.shallow(parameter) {
            Type::Var(var) => vec![var],
            function @ Type::Function(..) => self.free_vars(&function),
            _ => Vec::new(),
        }
    }

    fn parameter_types(members: &[(BindingId, &TypedExpr)]) -> Vec<Type> {
        members
            .iter()
            .flat_map(|(_, value)| match &value.kind {
                TypedExprKind::Fn(lambda) => {
                    lambda.params.iter().map(|param| param.ty.clone()).collect()
                }
                _ => Vec::new(),
            })
            .collect()
    }

    fn result_vars(&self, members: &[(BindingId, &TypedExpr)]) -> Vec<TypeVar> {
        members
            .iter()
            .filter_map(|(_, value)| match self.unifier.resolve(&value.ty) {
                Type::Function(_, _, result) => Some(result),
                _ => None,
            })
            .flat_map(|result| self.free_vars(&result))
            .collect()
    }

    fn component_vars(&self, members: &[(BindingId, &TypedExpr)]) -> Vec<TypeVar> {
        let candidates = self.generalizable_vars(members);
        if candidates.is_empty() {
            return candidates;
        }
        let excluded = self.monomorphic_vars();
        candidates
            .into_iter()
            .filter(|var| !excluded.contains(var))
            .collect()
    }

    fn generalizable_vars(&self, members: &[(BindingId, &TypedExpr)]) -> Vec<TypeVar> {
        let mut vars = Vec::new();
        for (binding, value) in members {
            if self.generalizes(binding, value) {
                self.add_free_vars(binding, &mut vars);
            }
        }
        vars
    }

    fn generalizes(&self, binding: &BindingId, value: &TypedExpr) -> bool {
        !self.signatures.contains_key(binding) && matches!(value.kind, TypedExprKind::Fn(_))
    }

    fn add_free_vars(&self, binding: &BindingId, vars: &mut Vec<TypeVar>) {
        for var in self.free_vars(&self.component[binding]) {
            if !vars.contains(&var) {
                vars.push(var);
            }
        }
    }

    fn generalize_vars(
        &mut self,
        vars: Vec<TypeVar>,
        start: usize,
        span: Span,
        pinnable_vars: &FxHashSet<TypeVar>,
    ) {
        let mut position = start;
        for var in vars {
            if self
                .generalize_var(var, position, span, pinnable_vars)
                .is_some()
            {
                position += 1;
            }
        }
    }

    fn link_round(&mut self, vars: &[TypeVar], span: Span) -> bool {
        let mut linked = false;
        for var in vars {
            if self.is_unbound(*var) {
                linked |= self.link_var(*var, vars, span);
            }
        }
        linked
    }

    fn link_var(&mut self, var: TypeVar, vars: &[TypeVar], span: Span) -> bool {
        let mut linked = false;
        for other in self.linked_vars(var, vars) {
            if self.is_unbound(var)
                && self.is_unbound(other)
                && self
                    .unifier
                    .unify(&Type::Var(var), &Type::Var(other), span)
                    .is_ok()
            {
                linked = true;
            }
        }
        linked
    }

    fn linked_vars(&self, var: TypeVar, vars: &[TypeVar]) -> Vec<TypeVar> {
        let table = &self.declarations.traits().table;
        self.unifier
            .bounds(var)
            .iter()
            .filter(|bound| table.links_argument(bound.binding))
            .filter_map(|bound| {
                let argument = bound
                    .argument
                    .as_ref()
                    .map(|argument| self.unifier.shallow(argument));
                match argument {
                    Some(Type::Var(other)) if other != var && vars.contains(&other) => Some(other),
                    _ => None,
                }
            })
            .collect()
    }

    fn is_unbound(&self, var: TypeVar) -> bool {
        matches!(self.unifier.shallow(&Type::Var(var)), Type::Var(current) if current == var)
    }

    fn mentioned_generics(&self, inferred: &Type) -> Vec<GenericParam> {
        let mut ids = Vec::new();
        Self::collect_generics(inferred, &mut ids);
        ids.into_iter()
            .filter_map(|id| self.generics.param(id).cloned())
            .collect()
    }

    fn collect_generics(value: &Type, ids: &mut Vec<GenericId>) {
        match value {
            Type::Generic(id) => {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
            _ => value
                .parts()
                .into_iter()
                .for_each(|part| Self::collect_generics(part, ids)),
        }
    }

    fn generalize_var(
        &mut self,
        var: TypeVar,
        position: usize,
        span: Span,
        pinnable_vars: &FxHashSet<TypeVar>,
    ) -> Option<GenericParam> {
        if !self.is_unbound(var) {
            return None;
        }
        let literal = self.unifier.literal_kind(var);
        if let Some(literal) = literal
            && (self.default_literal_parameters || !pinnable_vars.contains(&var))
        {
            self.default_literal(var, literal, span);
            return None;
        }
        let name = ParameterNames::at(position);
        let bounds = self.self_plain_bounds(var);
        let param = self.generic_declarations().generalize(var, &name, bounds);
        if let Some(literal) = literal {
            self.generics.mark_literal(param.id, literal);
        }
        Some(param)
    }

    pub(super) fn default_literal(&mut self, var: TypeVar, literal: LiteralKind, span: Span) {
        let defaulted = self.unifier.unify(
            &Type::Var(var),
            &Type::Primitive(literal.default_type()),
            span,
        );
        debug_assert!(
            defaulted.is_ok(),
            "a literal variable accepts its default type"
        );
    }

    pub(super) fn self_plain_bounds(&self, var: TypeVar) -> Vec<Bound> {
        let mut bounds = Vec::new();
        for bound in self.unifier.bounds(var) {
            let bound = match &bound.argument {
                Some(argument) => {
                    let argument = self.unifier.resolve(argument);
                    if argument == Type::Var(var) {
                        Bound::plain(bound.binding)
                    } else {
                        Bound::with_argument(bound.binding, argument)
                    }
                }
                None => bound.clone(),
            };
            if !bounds.contains(&bound) {
                bounds.push(bound);
            }
        }
        bounds
    }

    fn monomorphic_vars(&self) -> FxHashSet<TypeVar> {
        self.monomorphic
            .iter()
            .flat_map(|monomorphic| self.free_vars(monomorphic))
            .collect()
    }

    pub(super) fn free_vars(&self, value: &Type) -> Vec<TypeVar> {
        let mut vars = Vec::new();
        Self::collect_vars(&self.unifier.resolve(value), &mut vars);
        vars
    }

    fn collect_vars(value: &Type, vars: &mut Vec<TypeVar>) {
        match value {
            Type::Var(var) if !vars.contains(var) => vars.push(*var),
            Type::Function(_, parameters, result) => {
                parameters
                    .iter()
                    .for_each(|parameter| Self::collect_vars(parameter, vars));
                Self::collect_vars(result, vars);
            }
            Type::Named(_, arguments) => arguments
                .iter()
                .for_each(|argument| Self::collect_vars(argument, vars)),
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::collect_vars(inner, vars),
            _ => {}
        }
    }

    fn collect_user_trait(&mut self, declaration: UserTrait<'_>) -> Result<(), Diagnostic> {
        if declaration.takes_argument() {
            self.checking_facts
                .trait_arguments
                .insert(declaration.binding);
        }
        let collected = self.trait_collector().collect_user_trait(declaration)?;
        self.declarations.builder().traits().enter_trait(
            collected,
            &mut self.checking_facts,
            &mut self.synthetic_bindings,
        );
        Ok(())
    }

    fn collect_trait_impls(&mut self, modules: &[CoreModule]) {
        for module in modules {
            for item in &module.items {
                let ItemKind::Impl(trait_binding, implementor, associated, methods) = &item.kind
                else {
                    continue;
                };
                self.collect_trait_impl(ImplItem {
                    module: module.id,
                    trait_binding: *trait_binding,
                    implementor,
                    associated,
                    methods,
                });
            }
        }
    }

    fn collect_trait_impl(&mut self, item: ImplItem<'_>) {
        let lowered = match self
            .written_schemes()
            .lower_impl_implementor(item.implementor, item.associated)
        {
            Ok(lowered) => lowered,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                self.outcomes
                    .fail(item.methods.iter().map(|method| method.binding));
                return;
            }
        };
        let verdict = self
            .implementation_check()
            .verdict(item, lowered.implementor());
        match verdict {
            ImplementationVerdict::Rejected(rejection) => self.diagnostics.push(rejection),
            ImplementationVerdict::Accepted(problems) => {
                self.diagnostics.extend(problems);
                let collected = self.implementation_collector().collect(item, lowered);
                self.declarations.builder().traits().enter_impl(
                    collected,
                    &mut self.synthetic_bindings,
                    &mut self.impl_methods,
                );
            }
        }
    }

    fn implementation_check(&self) -> ImplementationCheck<'_> {
        ImplementationCheck::new(&self.bindings, &self.declarations, self.display())
    }

    fn implementation_collector(&self) -> ImplementationCollector<'_> {
        ImplementationCollector::new(self.declared_traits())
    }

    fn report_supertraits(&mut self) {
        let implementations: Vec<(BindingId, Type, Span)> = self
            .declarations
            .traits()
            .impls
            .iter()
            .map(|implementation| {
                (
                    implementation.trait_binding,
                    implementation.implementor.clone(),
                    implementation.implementor_span,
                )
            })
            .collect();
        for (trait_binding, implementor, span) in implementations {
            self.report_missing_supertraits(trait_binding, &implementor, span);
        }
    }

    fn report_missing_supertraits(
        &mut self,
        trait_binding: BindingId,
        implementor: &Type,
        span: Span,
    ) {
        let Some(rust_trait) = self.declarations.traits().table.get(trait_binding) else {
            return;
        };
        let supertraits = rust_trait.rows().reached().supertraits().to_vec();
        if supertraits.is_empty() {
            return;
        }
        let trait_name = self.bindings.get(trait_binding).name().to_string();
        let implementor_name = self.display().show(implementor);
        for supertrait in supertraits {
            if self.declarations.traits().table.satisfies(
                &supertrait,
                implementor,
                &|generic, bound| self.unifier.generic_bounds(generic).contains(bound),
            ) {
                continue;
            }
            let supertrait_name = self.bindings.get(supertrait.binding).name().to_string();
            let help = self
                .declarations
                .traits()
                .table
                .get(supertrait.binding)
                .and_then(|known| known.help.clone());
            self.diagnostics.push(TypeProblems::missing_supertrait(
                &trait_name,
                &supertrait_name,
                &implementor_name,
                span,
                help,
            ));
        }
    }

    fn acyclic_requirements(
        &mut self,
        declaration: &crate::core::RustTraitDeclaration,
    ) -> Result<Vec<Bound>, Diagnostic> {
        let mut supertraits = Vec::new();
        for required in &declaration.requires {
            let bound = self
                .written_types()
                .lower_type_bound(&required.bound, &FxHashMap::default())?;
            if self.leads_back(&bound, declaration.binding) {
                self.diagnostics.push(TypeProblems::requirement_cycle(
                    self.bindings.get(declaration.binding).name(),
                    self.bindings.get(bound.binding).name(),
                    required.span,
                ));
            } else {
                supertraits.push(bound);
            }
        }
        Ok(supertraits)
    }

    fn leads_back(&self, required: &Bound, declared: BindingId) -> bool {
        self.declarations
            .traits()
            .table
            .brings(required, &|brought| brought.binding == declared)
    }

    fn collect_rust_trait(
        &mut self,
        declaration: &crate::core::RustTraitDeclaration,
        reached: &ReachedTraits,
    ) -> Result<(), Diagnostic> {
        let rust_trait = if reached.reaches(declaration.binding) {
            let implementors = self.lower_rust_trait_implementors(declaration)?;
            let supertraits = self.acyclic_requirements(declaration)?;
            RustTrait::lowered(declaration, LoweredRows::new(implementors, supertraits))
        } else {
            RustTrait::unreached(declaration)
        };
        let collected = CollectedRustTrait::new(declaration, rust_trait);
        let traits = self.declarations.builder().traits();
        traits.enter_rust_trait(collected, &mut self.synthetic_bindings);
        Ok(())
    }

    fn check_generic_bound(&mut self, generic: GenericId, bound: Bound, span: Span) {
        if self.declares_bound(generic, &bound) {
            return;
        }
        if self.implied_bounds.is_impl_generic(generic) {
            self.implied_bounds.record(generic, bound);
            return;
        }
        if self.apply_declared_bound(generic, &bound, span) {
            return;
        }
        if self.implies(generic, &bound) {
            if self.declared_bound_implies(generic, &bound) {
                return;
            }
            self.implied_bounds.record(generic, bound);
            return;
        }
        if self.has_written_projection_on(generic, bound.binding) {
            return;
        }
        let generic_name = self.display().show(&Type::Generic(generic));
        let diagnostic = TypeProblems::missing_bound(
            self.bindings.get(bound.binding).name(),
            &generic_name,
            span,
        );
        self.diagnostics.push(diagnostic);
    }

    fn declares_bound(&self, generic: GenericId, bound: &Bound) -> bool {
        self.unifier
            .generic_bounds(generic)
            .iter()
            .any(|declared| declared.meets(bound, generic))
    }

    fn apply_declared_bound(&mut self, generic: GenericId, bound: &Bound, span: Span) -> bool {
        let Some(required) = &bound.argument else {
            return false;
        };
        let declared: Vec<Type> = self
            .unifier
            .generic_bounds(generic)
            .iter()
            .filter(|declared| declared.binding == bound.binding)
            .filter_map(|declared| declared.argument.clone())
            .collect();
        declared
            .into_iter()
            .any(|declared| self.unifier.unify(&declared, required, span).is_ok())
    }

    fn implies(&self, generic: GenericId, bound: &Bound) -> bool {
        if self.declared_bound_implies(generic, bound) {
            return true;
        }
        if bound.argument.is_some() {
            return false;
        }
        match self.admissible_types().of(generic) {
            Admissible::Closed(types) => {
                !types.is_empty()
                    && types.iter().all(|candidate| {
                        self.declarations
                            .traits()
                            .table
                            .satisfies(bound, candidate, &|_, _| false)
                    })
            }
            Admissible::Unbounded | Admissible::Open(_) => false,
        }
    }

    fn declared_bound_implies(&self, generic: GenericId, bound: &Bound) -> bool {
        self.unifier.generic_bounds(generic).iter().any(|declared| {
            self.declarations
                .traits()
                .table
                .implies(declared, bound, generic)
        })
    }

    pub(super) fn report_missing_trait(&mut self, bound: &Bound, implementor: &Type, span: Span) {
        let missing = MissingTrait {
            bound,
            implementor,
            span,
            shown_argument: self.literal_settlement().shown_argument(bound),
        };
        let diagnostic = self.missing_trait_explanation().explain(missing);
        self.diagnostics.push(diagnostic);
    }
}

impl TypeChecker {
    pub fn check(
        program: &crate::expand::Program,
        graph: &DefinitionGraph,
        changed_structs: &FxHashSet<BindingId>,
        inferred_copies: &InferredCopies,
        pinned: &FxHashMap<BindingId, Type>,
        literal_defaults: LiteralDefaults,
        default_literal_parameters: bool,
    ) -> Result<super::typed::TypedProgram, Diagnostics> {
        let configured = |vector_bases: &FxHashSet<Span>, checking| {
            let mut checker = Self::new(program.bindings.clone());
            checker.string_view = program.string_view;
            checker.checking = checking;
            checker.changed_structs = changed_structs.clone();
            checker.inferred_copies = inferred_copies.clone();
            checker.pinned = pinned.clone();
            checker.literal_defaults = literal_defaults.clone();
            checker.default_literal_parameters = default_literal_parameters;
            checker.vector_bases = vector_bases.clone();
            checker.call_origins = Rc::clone(&program.call_origins);
            checker
        };
        let mut vector_bases = FxHashSet::default();
        let mut checking = super::typed::CheckWork::default();
        loop {
            let (typed, open_collections) =
                configured(&vector_bases, checking).check_program(program, graph)?;
            checking = typed.checking;
            let found = open_collections.vector_bases(program, graph, &typed)?;
            if found.is_empty() {
                return Ok(typed);
            }
            vector_bases.extend(found);
        }
    }

    fn check_program(
        mut self,
        program: &crate::expand::Program,
        graph: &DefinitionGraph,
    ) -> Result<(super::typed::TypedProgram, OpenCollections), Diagnostics> {
        self.outcomes = DefinitionOutcomes::new(graph.reach().unreached());
        self.record_prelude_definitions(&program.modules, &program.pending_definitions);
        self.collect_declarations(&program.modules, graph);
        let mut definitions = self.check_definitions(program.entry.function, graph);
        let mut finalizer = super::finalize::Finalizer::new(&mut self);
        for binding in graph.order() {
            if let Some(definition) = definitions.get_mut(&binding) {
                finalizer.definition(definition);
            }
        }
        self.impl_bounds().absorb(&definitions);
        self.record_uses_through_impls();
        self.discharge_obligations(ObligationPass::Final);
        self.check_written_projections();
        self.resolve_missing_annotations();
        self.label_parameter_calls(&program.modules);
        self.check_opaque_positions(&program.modules, &definitions);
        let open_collections = self.open_collections();
        let typed_modules = program
            .modules
            .iter()
            .map(|module| Self::typed_module(module, &mut definitions))
            .collect();
        Ok((self.into_program(typed_modules, program)?, open_collections))
    }

    fn typed_module(
        module: &CoreModule,
        definitions: &mut FxHashMap<BindingId, TypedDefinition>,
    ) -> super::typed::TypedModule {
        let mut typed = super::typed::TypedModule {
            id: module.id,
            path: module.path.clone(),
            definitions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            exports: module.exports().bindings().collect(),
        };
        for item in &module.items {
            match &item.kind {
                ItemKind::Def(binding, _) => typed.definitions.extend(definitions.remove(binding)),
                ItemKind::Struct(binding, ..) => typed.structs.push(*binding),
                ItemKind::Enum(binding, ..) => typed.enums.push(*binding),
                ItemKind::Signature(..)
                | ItemKind::RustTrait(..)
                | ItemKind::RustType(..)
                | ItemKind::Trait(..)
                | ItemKind::Impl(..)
                | ItemKind::Derive(..)
                | ItemKind::MismatchHelp(..) => {}
            }
        }
        typed
    }

    fn into_program(
        self,
        modules: Vec<super::typed::TypedModule>,
        program: &crate::expand::Program,
    ) -> Result<super::typed::TypedProgram, Diagnostics> {
        let warnings = self.warnings;
        let mut diagnostics = self.diagnostics.distinct();
        if diagnostics.has_errors() {
            diagnostics.extend(self.unnameable_types);
            diagnostics.extend(warnings);
            return Err(diagnostics);
        }
        let generics = DeclaredGenerics::from(self.generics);
        Ok(super::typed::TypedProgram {
            modules,
            entry: program.entry.module,
            entry_function: program.entry.function,
            bindings: self.bindings,
            macro_field_reads: self.macro_field_reads,
            declarations: self.declarations,
            structural_uses: self.structural_uses,
            associated_generics: generics.associated_types,
            conversions: self.deferred.conversions,
            synthetic_bindings: self.synthetic_bindings,
            generics: generics.params,
            implied_bounds: self.implied_bounds.into_bounds(),
            template_borrows: self.template_borrows,
            template_clauses: self.template_clauses,
            pending_mode_checks: self.deferred.mode_checks,
            parameter_modes: ParameterModes::default(),
            stored_function_parameters: FxHashSet::default(),
            escaping_lambdas: FxHashSet::default(),
            borrowing_lambdas: FxHashMap::default(),
            mutable_function_bindings: FxHashSet::default(),
            literal_generics: generics.literals,
            unnameable_types: self.unnameable_types,
            closure_types: self.closure_types,
            type_texts: self.type_texts,
            warnings,
            checking: self.checking,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TypeChecker;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};
    use crate::source::{SourceId, Span};
    use crate::types::{Bound, Type, TypeVar};

    struct LinkingFixture {
        checker: TypeChecker,
        span: Span,
    }

    impl LinkingFixture {
        fn new() -> Self {
            Self {
                checker: TypeChecker::new(BindingArena::default()),
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn fresh(&mut self) -> TypeVar {
            match self.checker.unifier.fresh() {
                Type::Var(var) => var,
                other => panic!("a fresh type is a variable, found {other:?}"),
            }
        }

        fn bound_trait(&mut self) -> BindingId {
            self.checker.bindings.add(BindingInfo::new(
                "trait".into(),
                BindingKind::TypeVariable,
                None,
            ))
        }

        fn link(&mut self, var: TypeVar, other: TypeVar) -> bool {
            let binding = self.bound_trait();
            self.checker
                .unifier
                .add_bound(var, Bound::with_argument(binding, Type::Var(other)));
            self.checker.link_var(var, &[var, other], self.span)
        }
    }

    #[test]
    fn a_two_type_bound_links_its_variables() {
        let mut fixture = LinkingFixture::new();
        let var = fixture.fresh();
        let other = fixture.fresh();
        assert!(fixture.link(var, other));
        assert_eq!(
            fixture.checker.unifier.shallow(&Type::Var(var)),
            Type::Var(other)
        );
    }
}
