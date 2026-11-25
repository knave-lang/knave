use std::rc::Rc;

use super::shared_variables::SharedVariables;
use crate::core::{BindingId, BindingKind, Expr, TypeExpr, VariantInfo};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::types::assumed_collections::CollectionAssumption;
use crate::types::checker::TypeChecker;
use crate::types::collection::Collection;
use crate::types::expectation::Expectation;
use crate::types::forms::{
    AnnotationLowering, ArgumentTyping, Bindings, ChildTyping, CollectionAssumptions,
    CollectionLookups, CollectionShapes, ExpectedChildTyping, FieldMoves, GlobalReferences,
    LiteralUnknowns, LocalScope, LocalTypes, MacroOrigins, MapKeys, Obligations, OperandTyping,
    PlaceReads, RecurTargets, SchemeInstantiation, StructBases, SyntheticCalls, TemplateSchemes,
    TypeDisplays, TypeFitting, TypeTextRecords, TypeUnification, Unknowns, VariantArms,
    VariantSites,
};
use crate::types::problems::TypeProblems;
use crate::types::recur_target::RecurTarget;
use crate::types::synthetic_call::SyntheticCall;
use crate::types::synthetic_signature::SyntheticSignature;
use crate::types::templates::TemplateName;
use crate::types::ty::{Scheme, Type};
use crate::types::typed::{Borrow, TypedArgument, TypedExpr};
use crate::types::unifier::{LiteralKind, UnifyError};
use crate::types::variant_arm_reference::VariantArmReference;
use crate::types::variant_site::VariantSite;

impl ChildTyping for TypeChecker {
    fn type_child(&mut self, child: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.infer(child)
    }
}

impl ExpectedChildTyping for TypeChecker {
    fn type_child_against(
        &mut self,
        child: &Expr,
        expectation: Expectation,
    ) -> Result<TypedExpr, Diagnostic> {
        self.infer_against(child, expectation.expected, expectation.context)
    }
}

impl TypeFitting for TypeChecker {
    fn fit(
        &mut self,
        expectation: Expectation,
        found: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.unification().expect(expectation, found, span)
    }
}

impl MacroOrigins for TypeChecker {
    fn macro_name(&self, span: Span) -> Option<Rc<str>> {
        TypeChecker::macro_name(self, span)
    }
}

impl PlaceReads for TypeChecker {
    fn read_type(&self, value: &TypedExpr) -> Type {
        self.place_types().read(value)
    }

    fn changed_type(&self, value: &TypedExpr) -> Type {
        self.place_types().changed(value)
    }

    fn field_place(&self, value: &TypedExpr) -> Option<String> {
        self.passing_facts().field_place(value)
    }
}

impl Unknowns for TypeChecker {
    fn fresh(&mut self) -> Type {
        self.unifier.fresh()
    }

    fn resolve(&self, value: &Type) -> Type {
        self.unifier.resolve(value)
    }

    fn is_open(&self, value: &Type) -> bool {
        matches!(value, Type::Var(var) if self.unifier.literal_kind(*var).is_none())
    }
}

impl Obligations for TypeChecker {
    fn require_copy(&mut self, value: &Type) {
        TypeChecker::require_copy(self, value);
    }
}

impl VariantSites for TypeChecker {
    fn variant_site(&mut self, binding: BindingId, expected: Option<&Type>) -> VariantSite {
        TypeChecker::variant_site(self, binding, expected)
    }

    fn is_result_ok(&self, info: VariantInfo) -> bool {
        self.declarations
            .types()
            .enums
            .variant_shape(info)
            .is_result_ok()
    }
}

impl OperandTyping for TypeChecker {
    fn type_operand(&mut self, operand: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.within_operand(|checker| checker.infer(operand))
    }
}

impl TypeTextRecords for TypeChecker {
    fn note_type_text(&mut self) {
        self.holds_infer_forms = true;
    }
}

impl AnnotationLowering for TypeChecker {
    fn lower_annotation(&mut self, annotation: &TypeExpr) -> Result<Type, Diagnostic> {
        self.written_types().lower_without_generics(annotation)
    }
}

impl LocalScope for TypeChecker {
    fn bind_local(&mut self, binding: BindingId, ty: Type) {
        self.locals.insert(binding, ty);
    }
}

impl LocalTypes for TypeChecker {
    fn local_type(&self, binding: BindingId) -> Type {
        self.locals[&binding].clone()
    }
}

impl CollectionShapes for TypeChecker {
    fn nested_vector_of(&self, depth: usize, element: &Type) -> Type {
        self.prelude_types().nested_vector_of(depth, element)
    }

    fn slice_of(&self, element: Type) -> Type {
        self.prelude_types().slice_of(element)
    }
}

impl CollectionLookups for TypeChecker {
    fn collection_of(&self, ty: &Type) -> Option<Collection> {
        Collection::of(ty, &self.declarations.types().rust_types)
    }

    fn collection_of_referent(&self, ty: &Type) -> Option<Collection> {
        Collection::of_referent(ty, &self.declarations.types().rust_types)
    }
}

impl MapKeys for TypeChecker {
    fn require_map_key(&mut self, map: &Collection, span: Span) {
        self.check_map_key(map, span);
    }

    fn require_key_borrow(&mut self, map: &Collection, query: &Type, span: Span) {
        self.check_borrow(map, query, span);
    }
}

impl StructBases for TypeChecker {
    fn explain_struct_base(&self, error: Diagnostic, base: &Type) -> Diagnostic {
        self.point_struct_base_to_fields(error, base)
    }
}

impl FieldMoves for TypeChecker {
    fn moved_base(&mut self, base: TypedExpr) -> TypedExpr {
        self.moved_field_base(base)
    }
}

impl LiteralUnknowns for TypeChecker {
    fn fresh_literal(&mut self, kind: LiteralKind) -> Type {
        self.unifier.fresh_literal(kind)
    }
}

impl TemplateSchemes for TypeChecker {
    fn lower_template(&mut self, type_expr: &TypeExpr) -> Result<Scheme, Diagnostic> {
        self.written_schemes().lower_template(type_expr)
    }

    fn register_inline(&mut self, text: &Rc<str>, scheme: &Scheme) {
        let rust_types = &self.declarations.types().rust_types;
        self.template_borrows
            .register(TemplateName::Inline(text), scheme, rust_types);
    }
}

impl SchemeInstantiation for TypeChecker {
    fn instantiate(&mut self, scheme: &Scheme, span: Span) -> Type {
        self.instantiation().instantiate(scheme, span)
    }
}

impl Bindings for TypeChecker {
    fn binding_name(&self, binding: BindingId) -> String {
        self.bindings.get(binding).name().to_string()
    }

    fn binding_kind(&self, binding: BindingId) -> BindingKind {
        self.bindings.get(binding).kind()
    }
}

impl VariantArms for TypeChecker {
    fn variant_arm(&self, binding: BindingId) -> Option<VariantArmReference> {
        self.variant_arm.get(&binding).copied()
    }
}

impl GlobalReferences for TypeChecker {
    fn is_unchecked(&self, binding: BindingId) -> bool {
        self.unchecked.contains(&binding)
    }

    fn selfless_method_problem(
        &self,
        binding: BindingId,
        name: &str,
        span: Span,
    ) -> Option<Diagnostic> {
        let method = self.checking_facts.selfless_methods.get(binding)?;
        let scheme = self
            .schemes
            .get(&binding)
            .expect("a trait method has a scheme before any call");
        let replacement = self
            .display()
            .scheme_with_receiver(scheme, method.trait_binding());
        Some(TypeProblems::selfless_method_call(
            self.bindings.get(method.trait_binding()).name(),
            name,
            &replacement,
            span,
            method.signature(),
        ))
    }

    fn global_type(&mut self, binding: BindingId, span: Span) -> Type {
        TypeChecker::global_type(self, binding, span)
    }
}

impl RecurTargets for TypeChecker {
    fn recur_target(&self) -> RecurTarget {
        self.recur_target
            .clone()
            .expect("expansion checks that `recur` is inside a function")
    }
}

impl ArgumentTyping for TypeChecker {
    fn type_arguments(
        &mut self,
        callee: Option<&TypedExpr>,
        parameters: &[Type],
        arguments: &[Expr],
    ) -> Result<Vec<TypedArgument>, Diagnostic> {
        let shared =
            SharedVariables::of(parameters, &self.unifier, &self.declarations.traits().table);
        let mut typed_arguments = self.infer_arguments(callee, parameters, arguments, &shared)?;
        self.pass_shared_arguments(callee, &shared, &mut typed_arguments)?;
        Ok(typed_arguments)
    }
}

impl SyntheticCalls for TypeChecker {
    fn type_synthetic_call(&mut self, call: SyntheticCall<'_>) -> Result<TypedExpr, Diagnostic> {
        let SyntheticSignature {
            template,
            parameters,
            result,
        } = call.signature;
        self.synthetic_call_with_value(
            template,
            parameters,
            result,
            call.value,
            call.arguments,
            call.span,
        )
    }

    fn pass_argument(
        &mut self,
        position: usize,
        parameter: &Type,
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic> {
        self.pass_call_argument(None, position, parameter, value)
    }

    fn finish_call(
        &mut self,
        signature: SyntheticSignature<'_>,
        arguments: Vec<TypedArgument>,
        span: Span,
    ) -> TypedExpr {
        self.finish_synthetic_call(signature, arguments, span)
    }
}

impl TypeUnification for TypeChecker {
    fn unify(&mut self, left: &Type, right: &Type, span: Span) -> Result<(), UnifyError> {
        self.unifier.unify(left, right, span)
    }
}

impl TypeDisplays for TypeChecker {
    fn show(&self, ty: &Type) -> String {
        self.display().show(ty)
    }

    fn describe(&mut self, ty: &Type) -> String {
        self.unification().describe(ty)
    }
}

impl CollectionAssumptions for TypeChecker {
    fn assume_unknown_collection(&mut self, target: &TypedExpr, assumption: CollectionAssumption) {
        self.note_unknown_collection(target, assumption);
    }
}
