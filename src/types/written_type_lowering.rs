use rustc_hash::FxHashMap;

use super::checking_facts::CheckingFacts;
use super::declarations::Declarations;
use super::generic_declarations::GenericDeclarations;
use super::problems::TypeProblems;
use super::trait_boxing::TraitBoxing;
use super::ty::{Bound, GenericId, GenericParam, Lifetime, Type};
use super::written_projection::WrittenProjection;
use super::written_type_facts::WrittenTypeFacts;
use super::written_type_records::WrittenTypeRecords;
use crate::core::{
    BindingArena, BindingId, BindingKind, ClosureKind, ReferenceKind, TypeBound, TypeExpr,
    TypeExprKind,
};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;

pub(super) struct WrittenTypeLowering<'l> {
    bindings: &'l BindingArena,
    declarations: &'l Declarations,
    checking_facts: &'l CheckingFacts,
    type_arities: &'l FxHashMap<BindingId, usize>,
    generics: GenericDeclarations<'l>,
    projections: &'l mut Vec<WrittenProjection>,
    diagnostics: &'l mut Diagnostics,
}

struct ScopedTypeLowering<'a, 'l> {
    lowering: &'a mut WrittenTypeLowering<'l>,
    scope: &'a FxHashMap<BindingId, GenericId>,
}

impl<'l> WrittenTypeLowering<'l> {
    pub(super) fn new(
        facts: WrittenTypeFacts<'l>,
        generics: GenericDeclarations<'l>,
        records: WrittenTypeRecords<'l>,
    ) -> Self {
        Self {
            bindings: facts.bindings,
            declarations: facts.declarations,
            checking_facts: facts.checking_facts,
            type_arities: facts.type_arities,
            generics,
            projections: records.projections,
            diagnostics: records.diagnostics,
        }
    }

    pub(super) fn lower_type(
        &mut self,
        type_expr: &TypeExpr,
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<Type, Diagnostic> {
        ScopedTypeLowering {
            lowering: self,
            scope,
        }
        .lower(type_expr)
    }

    pub(super) fn lower_without_generics(
        &mut self,
        type_expr: &TypeExpr,
    ) -> Result<Type, Diagnostic> {
        self.lower_type(type_expr, &FxHashMap::default())
    }

    pub(super) fn lower_type_bound(
        &mut self,
        bound: &TypeBound,
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<Bound, Diagnostic> {
        let Some(argument) = &bound.argument else {
            return Ok(Bound::plain(bound.binding));
        };
        self.check_trait_argument(bound.binding, argument.span())?;
        let argument = self.lower_type(argument, scope)?;
        Ok(Bound::with_argument(bound.binding, argument))
    }

    pub(super) fn declare_variable(&mut self, variable: BindingId) -> GenericParam {
        let name = self.bindings.get(variable).name();
        match self.bindings.get(variable).kind() {
            BindingKind::AssociatedType => self.generics.declare_associated_type(name),
            _ => self.generics.declare(name, Vec::new()),
        }
    }

    pub(super) fn type_variables(&self, type_expr: &TypeExpr) -> Vec<BindingId> {
        let mut found = Vec::new();
        self.collect_type_variables(type_expr, &mut found);
        found
    }

    fn collect_type_variables(&self, type_expr: &TypeExpr, found: &mut Vec<BindingId>) {
        if let TypeExprKind::Named(binding, arguments) = type_expr.kind()
            && arguments.is_empty()
            && matches!(
                self.bindings.get(*binding).kind(),
                BindingKind::TypeVariable
            )
            && !found.contains(binding)
        {
            found.push(*binding);
        }
        for part in type_expr.parts() {
            self.collect_type_variables(part, found);
        }
    }

    pub(super) fn generics(&mut self) -> &mut GenericDeclarations<'l> {
        &mut self.generics
    }

    fn check_trait_argument(&self, binding: BindingId, span: Span) -> Result<(), Diagnostic> {
        if self.checking_facts.takes_argument(binding) {
            return Ok(());
        }
        if let Some(rust_trait) = self.declarations.traits.table.get(binding) {
            rust_trait.rows().assert_reached();
        }
        Err(TypeProblems::trait_takes_no_argument(
            self.bindings.get(binding).name(),
            span,
        ))
    }

    fn check_type_argument_count(
        &self,
        binding: BindingId,
        found: usize,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let expected = self.type_arities.get(&binding).copied().unwrap_or_default();
        if found == expected {
            return Ok(());
        }
        Err(TypeProblems::wrong_type_argument_count(
            self.bindings.get(binding).name(),
            expected,
            found,
            span,
        ))
    }

    fn check_boxable(&mut self, trait_binding: BindingId, span: Span) {
        let boxing = TraitBoxing::new(&self.declarations.traits, self.bindings);
        if let Some((blocker, help)) = boxing.blocker(trait_binding) {
            let trait_name = self.bindings.get(trait_binding).name();
            self.diagnostics
                .push(TypeProblems::not_boxable(trait_name, blocker, help, span));
        }
    }

    fn record_written_projection(&mut self, lowered: &Type, span: Span) {
        if let Type::Named(binding, arguments) = lowered
            && self
                .declarations
                .traits
                .table
                .projections()
                .is_projection(*binding)
        {
            self.projections.push(WrittenProjection {
                projection: *binding,
                owner: arguments[0].clone(),
                span,
            });
        }
    }
}

impl ScopedTypeLowering<'_, '_> {
    fn lower(&mut self, type_expr: &TypeExpr) -> Result<Type, Diagnostic> {
        self.lower_kind(type_expr.kind(), type_expr)
    }

    fn lower_kind(
        &mut self,
        kind: &TypeExprKind,
        type_expr: &TypeExpr,
    ) -> Result<Type, Diagnostic> {
        match kind {
            TypeExprKind::Where(..) | TypeExprKind::With(..) | TypeExprKind::Trait(_) => {
                self.lower_invalid(kind, type_expr)
            }
            _ => self.lower_plain(kind, type_expr),
        }
    }

    fn lower_plain(
        &mut self,
        kind: &TypeExprKind,
        type_expr: &TypeExpr,
    ) -> Result<Type, Diagnostic> {
        match kind {
            TypeExprKind::Primitive(primitive) => Ok(Type::Primitive(*primitive)),
            TypeExprKind::Named(binding, arguments) => {
                self.lower_named(*binding, arguments, type_expr.span())
            }
            TypeExprKind::Function(kind, parameters, result) => {
                self.lower_function(*kind, parameters, result)
            }
            TypeExprKind::Reference(kind, lifetime, inner) => {
                self.lower_reference(*kind, *lifetime, inner)
            }
            TypeExprKind::Hole => Ok(self.lowering.generics.fresh_variable()),
            _ => unreachable!("the caller matches a plain type"),
        }
    }

    fn lower_invalid(&self, kind: &TypeExprKind, type_expr: &TypeExpr) -> Result<Type, Diagnostic> {
        match kind {
            TypeExprKind::Where(..) => Err(TypeProblems::misplaced_where(type_expr.span())),
            TypeExprKind::With(..) => Err(TypeProblems::misplaced_with(type_expr.span())),
            TypeExprKind::Trait(binding) => Err(TypeProblems::trait_outside_box(
                self.lowering.bindings.get(*binding).name(),
                type_expr.span(),
            )),
            _ => unreachable!("the caller matches an invalid type"),
        }
    }

    fn lower_function(
        &mut self,
        kind: ClosureKind,
        parameters: &[TypeExpr],
        result: &TypeExpr,
    ) -> Result<Type, Diagnostic> {
        let parameters = parameters
            .iter()
            .map(|parameter| self.lower(parameter))
            .collect::<Result<Vec<Type>, Diagnostic>>()?;
        let result = self.lower(result)?;
        Ok(Type::Function(kind, parameters, Box::new(result)))
    }

    fn lower_reference(
        &mut self,
        kind: ReferenceKind,
        lifetime: Option<BindingId>,
        inner: &TypeExpr,
    ) -> Result<Type, Diagnostic> {
        let lifetime = lifetime.map_or(Lifetime::Elided, Lifetime::written);
        let inner = Box::new(self.lower(inner)?);
        Ok(match kind {
            ReferenceKind::Shared => Type::Ref(lifetime, inner),
            ReferenceKind::Mutable => Type::Mut(lifetime, inner),
        })
    }

    fn lower_named(
        &mut self,
        binding: BindingId,
        arguments: &[TypeExpr],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        self.lowering
            .check_type_argument_count(binding, arguments.len(), span)?;
        let arguments = self.lower_named_arguments(binding, arguments)?;
        let lowered = self.apply_named(binding, arguments);
        self.lowering.record_written_projection(&lowered, span);
        Ok(lowered)
    }

    fn lower_named_arguments(
        &mut self,
        binding: BindingId,
        arguments: &[TypeExpr],
    ) -> Result<Vec<Type>, Diagnostic> {
        arguments
            .iter()
            .map(|argument| {
                if self.lowering.declarations.types.rust_types.is_box(binding) {
                    self.lower_box_argument(argument)
                } else {
                    self.lower(argument)
                }
            })
            .collect()
    }

    fn apply_named(&mut self, binding: BindingId, arguments: Vec<Type>) -> Type {
        match (
            self.lowering.bindings.get(binding).kind(),
            self.scope.get(&binding),
        ) {
            (BindingKind::TypeVariable | BindingKind::AssociatedType, Some(generic)) => {
                Type::Generic(*generic)
            }
            (BindingKind::TypeVariable, None) => self.lowering.generics.fresh_variable(),
            _ => Type::Named(binding, arguments),
        }
    }

    fn lower_box_argument(&mut self, argument: &TypeExpr) -> Result<Type, Diagnostic> {
        match argument.kind() {
            TypeExprKind::Trait(binding) => {
                self.lowering.check_boxable(*binding, argument.span());
                Ok(Type::TraitObject(*binding))
            }
            _ => self.lower(argument),
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use crate::core::{PrimitiveType, ReferenceKind, TypeExprKind};
    use crate::types::ty::{Bound, GenericId, Type};
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;
    use crate::types::{LoweredRows, RustTrait};

    #[test]
    fn a_primitive_lowers_to_itself() {
        let mut fixture = Fixture::new();
        let lowered = fixture.lower_without_generics(&Fixture::integer());
        assert_eq!(
            lowered.ok(),
            Some(Type::Primitive(crate::core::PrimitiveType::I64))
        );
    }

    #[test]
    fn a_shared_reference_stays_shared() {
        let mut fixture = Fixture::new();
        let shared = Fixture::reference(ReferenceKind::Shared, Fixture::integer());
        let lowered = fixture.lower_without_generics(&shared);
        assert!(matches!(lowered, Ok(Type::Ref(..))));
    }

    #[test]
    fn a_mutable_reference_stays_mutable() {
        let mut fixture = Fixture::new();
        let mutable = Fixture::reference(ReferenceKind::Mutable, Fixture::integer());
        let lowered = fixture.lower_without_generics(&mutable);
        assert!(matches!(lowered, Ok(Type::Mut(..))));
    }

    #[test]
    fn a_function_lowers_its_parameters_and_result() {
        let mut fixture = Fixture::new();
        let boolean = Fixture::expression(TypeExprKind::Primitive(PrimitiveType::Bool));
        let function = Fixture::function(vec![Fixture::integer()], boolean);
        let Ok(Type::Function(_, parameters, result)) = fixture.lower_without_generics(&function)
        else {
            panic!("a function type lowers to a function");
        };
        assert_eq!(parameters, vec![Type::Primitive(PrimitiveType::I64)]);
        assert_eq!(*result, Type::Primitive(PrimitiveType::Bool));
    }

    #[test]
    fn a_hole_lowers_to_a_fresh_variable() {
        let mut fixture = Fixture::new();
        let hole = Fixture::expression(TypeExprKind::Hole);
        assert!(matches!(
            fixture.lower_without_generics(&hole),
            Ok(Type::Var(_))
        ));
    }

    #[test]
    fn a_type_variable_in_scope_lowers_to_its_generic() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("a");
        let generic = GenericId::new(0);
        let scope = FxHashMap::from_iter([(variable, generic)]);
        let lowered = fixture.lower(&Fixture::named(variable, Vec::new()), &scope);
        assert_eq!(lowered.ok(), Some(Type::Generic(generic)));
    }

    #[test]
    fn a_type_variable_without_generics_lowers_to_a_fresh_variable() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("a");
        let lowered = fixture.lower_without_generics(&Fixture::named(variable, Vec::new()));
        assert!(matches!(lowered, Ok(Type::Var(_))));
    }

    #[test]
    fn a_struct_with_its_declared_arity_lowers_to_a_named_type() {
        let mut fixture = Fixture::new();
        let wrapper = fixture.declared_struct("wrapper", 1);
        let written = Fixture::named(wrapper, vec![Fixture::integer()]);
        let lowered = fixture.lower_without_generics(&written);
        assert!(matches!(lowered, Ok(Type::Named(binding, _)) if binding == wrapper));
    }

    #[test]
    fn a_struct_with_too_many_arguments_is_refused() {
        let mut fixture = Fixture::new();
        let point = fixture.declared_struct("point", 0);
        let written = Fixture::named(point, vec![Fixture::integer()]);
        assert!(fixture.lower_without_generics(&written).is_err());
    }

    #[test]
    fn a_where_inside_a_type_is_refused() {
        let mut fixture = Fixture::new();
        let nested = Fixture::expression(TypeExprKind::Where(
            Vec::new(),
            Box::new(Fixture::integer()),
        ));
        assert!(fixture.lower_without_generics(&nested).is_err());
    }

    #[test]
    fn a_trait_outside_a_box_is_refused() {
        let mut fixture = Fixture::new();
        let display = fixture.named_trait("display");
        let bare = Fixture::expression(TypeExprKind::Trait(display));
        assert!(fixture.lower_without_generics(&bare).is_err());
    }

    #[test]
    fn a_bound_without_an_argument_lowers_to_a_plain_bound() {
        let mut fixture = Fixture::new();
        let display = fixture.named_trait("display");
        let binder = Fixture::bounded(display, &[display]);
        let bound = fixture.lower_bound(&binder.bounds[0]);
        assert_eq!(bound.ok(), Some(Bound::plain(display)));
    }

    #[test]
    fn a_declared_associated_type_is_named_in_the_declared_generics() {
        let mut fixture = Fixture::new();
        let item = fixture.associated_type("item");
        let param = fixture.checker.written_types().declare_variable(item);
        let declared = fixture.declared_generics();
        assert_eq!(
            declared.associated_types.get(&param.id).map(|name| &**name),
            Some("item")
        );
    }

    #[test]
    fn a_declared_type_variable_names_no_associated_type() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("a");
        let param = fixture.checker.written_types().declare_variable(variable);
        assert_eq!(&*param.name, "a");
        assert!(fixture.declared_generics().associated_types.is_empty());
    }

    #[test]
    fn replaced_bounds_reach_the_param_and_the_unifier() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("a");
        let display = fixture.named_trait("display");
        let mut lowering = fixture.checker.written_types();
        let mut param = lowering.declare_variable(variable);
        lowering
            .generics()
            .redeclare(&mut param, vec![Bound::plain(display)]);
        assert_eq!(param.bounds, vec![Bound::plain(display)]);
        assert_eq!(
            fixture.checker.unifier.generic_bounds(param.id),
            [Bound::plain(display)]
        );
    }

    #[test]
    fn a_reached_trait_without_argument_rows_takes_no_argument() {
        let mut fixture = Fixture::new();
        let shown = fixture.holding_trait(|declaration| {
            RustTrait::lowered(declaration, LoweredRows::new(Vec::new(), Vec::new()))
        });
        assert!(
            fixture
                .lower_bound(&Fixture::bound_with_argument(shown))
                .is_err()
        );
    }

    #[test]
    #[should_panic(expected = "the trait reach left out a trait the program uses")]
    fn an_argument_on_an_unreached_trait_fails_loudly() {
        let mut fixture = Fixture::new();
        let shown = fixture.holding_trait(RustTrait::unreached);
        let _ = fixture.lower_bound(&Fixture::bound_with_argument(shown));
    }

    #[test]
    fn the_type_variables_of_a_type_are_listed_once_in_order() {
        let mut fixture = Fixture::new();
        let first = fixture.type_variable("a");
        let second = fixture.type_variable("b");
        let point = fixture.declared_struct("point", 0);
        let triple = fixture.declared_struct("triple", 4);
        let written = Fixture::named(
            triple,
            vec![
                Fixture::named(first, Vec::new()),
                Fixture::named(point, Vec::new()),
                Fixture::named(second, Vec::new()),
                Fixture::named(first, Vec::new()),
            ],
        );
        let variables = fixture.checker.written_types().type_variables(&written);
        assert_eq!(variables, vec![first, second]);
    }

    #[test]
    fn a_type_variable_applied_to_arguments_is_no_type_variable() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("a");
        let written = Fixture::named(variable, vec![Fixture::integer()]);
        assert!(
            fixture
                .checker
                .written_types()
                .type_variables(&written)
                .is_empty()
        );
    }

    #[test]
    fn an_associated_type_is_no_type_variable() {
        let mut fixture = Fixture::new();
        let item = fixture.associated_type("item");
        let written = Fixture::named(item, Vec::new());
        assert!(
            fixture
                .checker
                .written_types()
                .type_variables(&written)
                .is_empty()
        );
    }
}
