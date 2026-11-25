mod form_capabilities;
mod labeled_variant;
mod open_collections;
mod shared_variables;
mod short_pattern;
mod variant_target;

use super::forms::{
    ArgumentTyping, Association, Body, Borrowing, Conditional, Dereference, ExpressionForm,
    IfOperands, InlineRust, LiteralValue, MapWrite, Recurrence, Sequence, StringPush,
    TypeTextRequest, VariableReference, VariantConstruction, VectorPush, VectorWrite,
    WriteCollection,
};
use super::recur_target::RecurTarget;
use super::result_span::ResultSpan;
use super::variant_site::VariantSite;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::assumed_collection_problems::AssumedCollectionProblems;
use super::assumed_collections::CollectionAssumption;
use super::borrowed_items::ArgumentSite;
use super::checker::TypeChecker;
use super::collection::{Collection, CollectionKind};
use super::enums::VariantPayload;
use super::expectation::Expectation;
use super::field_owner_problems::FieldOwnerProblems;
use super::field_read_expr::FieldReadExpr;
use super::inferred_span::InferredSpan;
use super::parameter_call_request::ParameterCallRequest;
use super::pending::DecidedBorrows;
use super::positional_read::PositionalRead;
use super::problems::{MismatchContext, TypeProblems};
use super::synthetic_signature::SyntheticSignature;
use super::synthetic_template::SyntheticTemplate;
use super::ty::{Bound, Type};
use super::typed::{
    Borrow, PendingModeCheck, TryErrorConversion, TypedArgument, TypedArm, TypedBinder,
    TypedBodyItem, TypedControl, TypedConversion, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf,
    TypedMatch, TypedPattern, TypedPatternKind, TypedStructExpr,
};
use super::variant_arm_reference::VariantArmReference;
use super::{BoxedPayload, PayloadPosition, variant_names::VariantNames};
use super::{pattern_fields::PatternFields, pattern_placement::PatternPlacement};
use crate::core::{
    Arm, Binder, BindingId, BindingKind, BorrowTrait, Control, EqualityTrait, Expr, ExprKind,
    FieldLabel, HashTrait, KeyedPath, Lambda, Leaf, Literal, MapOperation, OrderTrait, Pattern,
    PatternArguments, PatternKind, PrimitiveType, Push, ReferenceKind, RustTemplate, StructExpr,
    TypeExpr, TypeExprKind, VariantInfo,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode, SimilarName};
use crate::source::Span;
use shared_variables::SharedVariables;
use short_pattern::ShortPattern;
use variant_target::VariantTarget;

enum MapIterator {
    Keys,
    Vals,
}

struct ExpectedFunction<'a> {
    parameters: &'a [Type],
    result: &'a Type,
    context: MismatchContext,
    written: bool,
}

struct BareNamePattern<'a> {
    binding: BindingId,
    name: &'a str,
    candidates: &'a [BindingId],
    outer: Option<BindingId>,
    matched: Type,
    mode: Option<ReferenceKind>,
    span: Span,
}

#[derive(Clone)]
pub(super) struct TryTarget {
    returns: Type,
}

#[derive(Clone, Copy, PartialEq)]
enum TrySourceKind {
    Result,
    Option,
}

struct TrySource {
    kind: TrySourceKind,
    binding: BindingId,
    value: Type,
    error: Option<Type>,
}

struct TryConversion {
    enum_binding: BindingId,
    variant: usize,
    payload: Type,
}

impl TrySourceKind {
    fn needed(self, name: &str) -> String {
        match self {
            TrySourceKind::Result => {
                format!("`{name}` whose error can hold this value's error")
            }
            TrySourceKind::Option => format!("`{name}`"),
        }
    }
}

impl TypeChecker {
    fn infer(&mut self, expr: &Expr) -> Result<TypedExpr, Diagnostic> {
        self.checking.inferred_expressions += 1;
        let span = expr.span();
        InferredSpan::around(self, span, |checker| match expr.kind() {
            ExprKind::Leaf(leaf) => checker.infer_leaf(leaf, span),
            ExprKind::Fn(lambda) => Self::typed(checker.infer_lambda(lambda, span, None), span),
            ExprKind::Call(callee, arguments) => checker.infer_call(callee, arguments, span),
            ExprKind::Control(control) => checker.infer_control(control, span),
            other => checker.infer_nested(other, span),
        })
    }

    fn infer_nested(&mut self, kind: &ExprKind, span: Span) -> Result<TypedExpr, Diagnostic> {
        match kind {
            ExprKind::Recur(arguments) => Recurrence::new(arguments, span).type_against(self, None),
            ExprKind::Try(value) => self.infer_try(value, span),
            ExprKind::Infer(operand) => {
                TypeTextRequest::new(operand, span).type_against(self, None)
            }
            ExprKind::Borrow(kind, value) => {
                Borrowing::new(*kind, value, span).type_against(self, None)
            }
            ExprKind::Deref(value) => Dereference::new(value, span).type_against(self, None),
            _ => self.infer_composite(kind, span),
        }
    }

    fn infer_composite(&mut self, kind: &ExprKind, span: Span) -> Result<TypedExpr, Diagnostic> {
        match kind {
            ExprKind::Struct(structure) => self.infer_struct(structure, span),
            ExprKind::Variant(binding, arguments) => {
                VariantConstruction::new(*binding, arguments, span).type_against(self, None)
            }
            ExprKind::LabeledVariant(binding, fields) => {
                self.infer_labeled_variant(*binding, fields, None, span)
            }
            _ => self.infer_collection_form(kind, span),
        }
    }

    fn infer_collection_form(
        &mut self,
        kind: &ExprKind,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        match kind {
            ExprKind::Assoc(path) => Association::new(path, span).type_against(self, None),
            ExprKind::MapAt(operation, value, arguments) => {
                self.infer_map_at(*operation, value, arguments, span)
            }
            ExprKind::PushAt(push) => self.infer_push(push, span),
            ExprKind::SetAt(path) => self.infer_set_at(path, span),
            _ => unreachable!("the caller matches a nested expression"),
        }
    }

    fn infer_leaf(&mut self, leaf: &Leaf, span: Span) -> Result<TypedExpr, Diagnostic> {
        match leaf {
            Leaf::Literal(literal) => LiteralValue::new(literal, span).type_against(self, None),
            Leaf::Var(binding) => VariableReference::new(*binding, span).type_against(self, None),
            Leaf::Quote(_) => Err(TypeProblems::quote_at_runtime(span)),
            Leaf::Rust(template) => InlineRust::new(template, span).type_against(self, None),
        }
    }

    fn infer_control(&mut self, control: &Control, span: Span) -> Result<TypedExpr, Diagnostic> {
        match control {
            Control::Block(block) => Sequence::new(block, span).type_against(self, None),
            Control::If(test, then, otherwise) => {
                let operands = IfOperands {
                    test,
                    then,
                    otherwise,
                };
                Conditional::new(operands, span).type_against(self, None)
            }
            Control::Match(value, arms) => self.infer_match(value, arms, span),
        }
    }

    fn infer_struct(
        &mut self,
        structure: &StructExpr,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        match structure {
            StructExpr::New(binding, fields) => {
                Self::typed(self.infer_new(*binding, fields, span), span)
            }
            StructExpr::Update(base, fields) => {
                Self::typed(self.infer_update(base, fields, span), span)
            }
            StructExpr::Field(value, label) => self.infer_field(value, label, span),
        }
    }

    fn variant_site(&mut self, binding: BindingId, expected: Option<&Type>) -> VariantSite {
        let info = self.variant_info(binding);
        let type_arguments = match expected.map(|expected| self.unifier.shallow(expected)) {
            Some(Type::Named(expected_owner, arguments)) if expected_owner == info.owner => {
                arguments
            }
            _ => self.fresh_arguments(info.owner),
        };
        let shape = self.declarations.types().enums.variant_shape(info);
        let stored = shape.storage(&type_arguments, &mut self.unifier);
        let payload = self
            .variant_payloads()
            .seen_by_patterns(info.owner, info.index, stored);
        VariantSite {
            info,
            type_arguments,
            payload,
        }
    }

    fn infer_match(
        &mut self,
        value: &Expr,
        arms: &[Arm],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        self.infer_match_expecting(value, arms, None, span)
    }

    fn infer_match_expecting(
        &mut self,
        value: &Expr,
        arms: &[Arm],
        expected: Option<&Type>,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = self.infer(value)?;
        self.bind_match_enum(&value.ty, arms, value.span)?;
        let pattern_type = self.pattern_type(&value.ty, arms);
        let mut typed_arms = Vec::new();
        let mut result: Option<Type> = None;
        for arm in arms {
            let (mut typed_arm, body_type) = self.infer_arm(arm, &pattern_type, expected)?;
            let span = ResultSpan::of_block(&typed_arm.body).unwrap_or(typed_arm.span);
            self.record_arm_result(
                &mut result,
                &mut typed_arms,
                &mut typed_arm,
                &body_type,
                span,
            )?;
            typed_arms.push(typed_arm);
        }
        let verdict = self.match_coverage().judge(&value.ty, &typed_arms);
        verdict.report(span, &mut self.diagnostics, &mut self.warnings);
        let unguarded_exhaustive = verdict.is_exhaustive_without_guards();
        Ok(TypedExpr {
            kind: TypedExprKind::Control(TypedControl::Match(TypedMatch {
                value: Box::new(value),
                arms: typed_arms,
                unguarded_exhaustive,
            })),
            ty: result.unwrap_or_else(Type::unit),
            span,
        })
    }

    fn infer_arm(
        &mut self,
        arm: &Arm,
        pattern_type: &Type,
        expected: Option<&Type>,
    ) -> Result<(TypedArm, Type), Diagnostic> {
        let pattern = self.lower_pattern(&arm.pattern, pattern_type, None)?;
        let tail = expected.map(Expectation::plain);
        let (body, body_type) = Body::new(&arm.body).type_with(self, tail)?;
        Ok((
            TypedArm {
                pattern,
                body,
                span: arm.span,
            },
            body_type,
        ))
    }

    fn record_arm_result(
        &mut self,
        result: &mut Option<Type>,
        earlier: &mut [TypedArm],
        arm: &mut TypedArm,
        body_type: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Some(first) = result.clone() else {
            *result = Some(body_type.clone());
            return Ok(());
        };
        if let Some(target) = self.copy_target(&first, body_type, span) {
            for earlier_arm in earlier.iter_mut() {
                self.copy_arm_out(earlier_arm, &target);
            }
            *result = Some(target);
            return Ok(());
        }
        if let Some(target) = self.copy_target(body_type, &first, span) {
            self.copy_arm_out(arm, &target);
            *result = Some(target);
            return Ok(());
        }
        self.unification()
            .expect(Expectation::plain(&first), body_type, span)
    }

    fn copy_target(&mut self, reference: &Type, value: &Type, span: Span) -> Option<Type> {
        let Type::Ref(_, inner) = self.unifier.resolve(reference) else {
            return None;
        };
        let value = self.unifier.resolve(value);
        let literal = self.is_literal(&value);
        if !literal && !self.copy_rules().is_small_copy(&value) {
            return None;
        }
        self.unifier.unify(inner.as_ref(), &value, span).ok()?;
        if literal && matches!(inner.as_ref(), Type::Var(_)) {
            return Some(value);
        }
        let target = self.unifier.resolve(&value);
        self.copy_rules().is_small_copy(&target).then_some(target)
    }

    fn is_literal(&self, value: &Type) -> bool {
        matches!(value, Type::Var(var) if self.unifier.literal_kind(*var).is_some())
    }

    fn copy_arm_out(&self, arm: &mut TypedArm, expected: &Type) {
        let Some(TypedBodyItem::Expr(value)) = arm.body.items.last_mut() else {
            return;
        };
        let span = value.span;
        *value = TypedExpr {
            kind: TypedExprKind::Deref(Box::new(value.clone())),
            ty: expected.clone(),
            span,
        };
    }

    fn bind_match_enum(
        &mut self,
        scrutinee: &Type,
        arms: &[Arm],
        span: Span,
    ) -> Result<Option<BindingId>, Diagnostic> {
        let (matched, _) = self.reference_peeling().peel(scrutinee);
        match self.unifier.shallow(&matched) {
            Type::Named(owner, _) if self.declarations.types().enums.get(owner).is_some() => {
                Ok(Some(owner))
            }
            Type::Var(_) => {
                let Some(owner) = self.unknown_match_enum(arms)? else {
                    return Ok(None);
                };
                let expected = Type::Named(owner, self.fresh_arguments(owner));
                self.unification()
                    .expect(Expectation::plain(&expected), &matched, span)?;
                Ok(Some(owner))
            }
            _ => Ok(None),
        }
    }

    fn pattern_type(&self, scrutinee: &Type, arms: &[Arm]) -> Type {
        let strings = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, PatternKind::Literal(Literal::String(_))));
        if strings && Self::is_string_value(scrutinee) {
            Type::str_ref()
        } else {
            scrutinee.clone()
        }
    }

    fn is_string_value(ty: &Type) -> bool {
        match ty {
            Type::Primitive(PrimitiveType::String) => true,
            Type::Ref(_, inner) => matches!(inner.as_ref(), Type::Primitive(PrimitiveType::String)),
            _ => false,
        }
    }

    fn unknown_match_enum(&self, arms: &[Arm]) -> Result<Option<BindingId>, Diagnostic> {
        let mut fitting: Option<Vec<BindingId>> = None;
        let mut first: Option<(Rc<str>, Span)> = None;
        for arm in arms {
            self.collect_arm_candidates(&arm.pattern, &mut fitting, &mut first);
        }
        let Some(mut fitting) = fitting else {
            return Ok(None);
        };
        fitting.sort();
        fitting.dedup();
        match fitting.as_slice() {
            [] => Ok(None),
            [owner] => Ok(Some(*owner)),
            _ => {
                let (name, span) = first.expect("a fitting enum comes from a variant pattern");
                let names: Vec<String> = fitting
                    .iter()
                    .map(|owner| self.bindings.get(*owner).name().to_string())
                    .collect();
                Err(TypeProblems::ambiguous_variant(&name, &names, span))
            }
        }
    }

    fn collect_arm_candidates(
        &self,
        pattern: &Pattern,
        fitting: &mut Option<Vec<BindingId>>,
        first: &mut Option<(Rc<str>, Span)>,
    ) {
        let (name, candidates): (&str, Vec<BindingId>) = match &pattern.kind {
            PatternKind::Variant(binding, _) => (self.variant_name(*binding), vec![*binding]),
            PatternKind::ShortVariant {
                name, candidates, ..
            }
            | PatternKind::BareName {
                name, candidates, ..
            } => (name, candidates.clone()),
            _ => return,
        };
        let owners: Vec<BindingId> = candidates
            .iter()
            .map(|binding| self.variant_info(*binding).owner)
            .collect();
        *fitting = Some(match fitting.take() {
            Some(current) => current
                .into_iter()
                .filter(|owner| owners.contains(owner))
                .collect(),
            None => owners,
        });
        first.get_or_insert_with(|| (name.into(), pattern.span));
    }

    fn variant_info(&self, binding: BindingId) -> VariantInfo {
        match self.bindings.get(binding).kind() {
            BindingKind::Variant(_, info) => info,
            _ => unreachable!("a variant pattern names a variant binding"),
        }
    }

    fn variant_name(&self, binding: BindingId) -> &str {
        self.bindings
            .get(binding)
            .name()
            .rsplit('/')
            .next()
            .unwrap_or_default()
    }

    fn lower_pattern(
        &mut self,
        pattern: &Pattern,
        expected: &Type,
        inherited: Option<ReferenceKind>,
    ) -> Result<TypedPattern, Diagnostic> {
        let (matched, own_mode) = self.reference_peeling().peel(expected);
        let mode = own_mode.or(inherited);
        let kind = match &pattern.kind {
            PatternKind::Wildcard => Ok(TypedPatternKind::Wildcard),
            PatternKind::Bind(binding) => {
                Ok(self.bind_pattern(*binding, matched, mode, pattern.span))
            }
            PatternKind::BareName {
                name,
                binding,
                candidates,
                outer,
            } => self.bare_name_pattern(BareNamePattern {
                binding: *binding,
                name,
                candidates,
                outer: *outer,
                matched,
                mode,
                span: pattern.span,
            }),
            PatternKind::Variant(binding, patterns) => {
                self.named_variant_pattern(*binding, patterns, matched, mode, pattern.span)
            }
            PatternKind::Struct(binding, patterns) => {
                self.struct_pattern(*binding, patterns, matched, mode, pattern.span)
            }
            PatternKind::ShortVariant {
                name,
                candidates,
                arguments,
            } => {
                let short = ShortPattern {
                    name,
                    candidates,
                    written: arguments,
                    span: pattern.span,
                };
                self.short_variant_pattern(&short, matched, mode)
            }
            PatternKind::Literal(literal) => {
                self.literal_pattern(literal, &matched, expected, pattern.span)
            }
        };
        kind.map(|kind| TypedPattern {
            kind,
            boxed: false,
            span: pattern.span,
        })
    }

    fn bare_name_pattern(
        &mut self,
        pattern: BareNamePattern<'_>,
    ) -> Result<TypedPatternKind, Diagnostic> {
        match self.nullary_variant(
            pattern.name,
            pattern.candidates,
            &pattern.matched,
            pattern.span,
        )? {
            Some(variant) => {
                self.variant_arm.insert(
                    pattern.binding,
                    VariantArmReference {
                        outer: pattern.outer,
                        variant,
                    },
                );
                Ok(TypedPatternKind::Variant(variant, Vec::new()))
            }
            None => {
                Ok(self.bind_pattern(pattern.binding, pattern.matched, pattern.mode, pattern.span))
            }
        }
    }

    fn nullary_variant(
        &mut self,
        name: &str,
        candidates: &[BindingId],
        matched: &Type,
        span: Span,
    ) -> Result<Option<BindingId>, Diagnostic> {
        if let Type::Named(owner, _) = self.unifier.shallow(matched)
            && self.declarations.types().enums.get(owner).is_some()
        {
            return Ok(self.find_nullary_variant(owner, name, candidates));
        }
        if !matches!(self.unifier.shallow(matched), Type::Var(_)) {
            return Ok(None);
        }
        let candidate = self.unique_candidate(candidates, span)?;
        let owner = self.variant_info(candidate).owner;
        let arguments = self.fresh_arguments(owner);
        let expected = Type::Named(owner, arguments);
        self.unification()
            .expect(Expectation::plain(&expected), matched, span)?;
        Ok(self.find_nullary_variant(owner, name, candidates))
    }

    fn find_nullary_variant(
        &self,
        owner: BindingId,
        name: &str,
        candidates: &[BindingId],
    ) -> Option<BindingId> {
        let definition = self.declarations.types().enums.get(owner)?;
        let variant = definition
            .variants
            .iter()
            .find(|variant| variant.name.as_ref() == name)?;
        if !matches!(variant.payload, super::enums::VariantPayload::None) {
            return None;
        }
        candidates.iter().copied().find(|binding| {
            matches!(
                self.bindings.get(*binding).kind(),
                BindingKind::Variant(_, info) if info.owner == owner
            )
        })
    }

    fn literal_pattern(
        &mut self,
        literal: &Literal,
        matched: &Type,
        expected: &Type,
        span: Span,
    ) -> Result<TypedPatternKind, Diagnostic> {
        let literal_type = LiteralValue::new(literal, span).literal_type(self);
        let matched = self.unifier.resolve(matched);
        if matches!(literal, Literal::String(_)) && Self::is_string_position(&matched) {
            return Ok(TypedPatternKind::Literal(literal.clone(), matched));
        }
        let compared = match literal {
            Literal::String(_) => expected,
            _ => &matched,
        };
        self.unification()
            .expect(Expectation::plain(compared), &literal_type, span)?;
        Ok(TypedPatternKind::Literal(
            literal.clone(),
            self.unifier.resolve(&literal_type),
        ))
    }

    fn is_string_position(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Primitive(PrimitiveType::String | PrimitiveType::Str)
        )
    }

    fn bind_pattern(
        &mut self,
        binding: BindingId,
        matched: Type,
        mode: Option<ReferenceKind>,
        span: Span,
    ) -> TypedPatternKind {
        let ty = Self::binding_mode_type(matched, mode);
        self.locals.insert(binding, ty.clone());
        TypedPatternKind::Bind(TypedBinder::new(binding, ty, span))
    }

    fn named_variant_pattern(
        &mut self,
        binding: BindingId,
        written: &PatternArguments,
        matched: Type,
        mode: Option<ReferenceKind>,
        span: Span,
    ) -> Result<TypedPatternKind, Diagnostic> {
        let info = self.variant_info(binding);
        let arguments = self.match_arguments(matched, info.owner, span)?;
        let target = VariantTarget::new(binding, info, arguments);
        self.variant_target_pattern(target, PatternPlacement::new(written, span), mode)
    }

    fn variant_target_pattern(
        &mut self,
        target: VariantTarget,
        placement: PatternPlacement<'_>,
        mode: Option<ReferenceKind>,
    ) -> Result<TypedPatternKind, Diagnostic> {
        let VariantTarget { binding, info, .. } = target;
        let shape = self.declarations.types().enums.variant_shape(info);
        let storage = shape.storage(&target.arguments, &mut self.unifier);
        let placed = placement.place(self.variant_name(binding), &shape.pattern_fields())?;
        drop(shape);
        let typed = self.lower_payload_patterns(info.owner, info.index, &placed, &storage, mode)?;
        Ok(TypedPatternKind::Variant(binding, typed))
    }

    fn struct_pattern(
        &mut self,
        binding: BindingId,
        written: &PatternArguments,
        matched: Type,
        mode: Option<ReferenceKind>,
        span: Span,
    ) -> Result<TypedPatternKind, Diagnostic> {
        let arguments = self.match_arguments(matched, binding, span)?;
        let fields = self
            .declarations
            .types()
            .structs
            .get(binding)
            .map(|definition| definition.instantiated_fields(&arguments))
            .unwrap_or_default();
        let names = fields.iter().map(|(name, _)| name.as_ref()).collect();
        let placement = PatternPlacement::new(written, span);
        let placed = placement.place(
            self.bindings.get(binding).name(),
            &PatternFields::Named(names),
        )?;
        let typed = placed
            .into_iter()
            .zip(&fields)
            .map(|(pattern, (_, field_type))| self.lower_pattern(pattern, field_type, mode))
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        Ok(TypedPatternKind::Struct(binding, typed))
    }

    fn short_variant_pattern(
        &mut self,
        short: &ShortPattern<'_>,
        matched: Type,
        mode: Option<ReferenceKind>,
    ) -> Result<TypedPatternKind, Diagnostic> {
        let target = self.short_pattern_target(short, &matched)?;
        let placement = PatternPlacement::new(short.written, short.span);
        self.variant_target_pattern(target, placement, mode)
    }

    fn lower_payload_patterns(
        &mut self,
        owner: BindingId,
        variant: usize,
        patterns: &[&Pattern],
        storage: &[Type],
        mode: Option<ReferenceKind>,
    ) -> Result<Vec<TypedPattern>, Diagnostic> {
        let mut typed = Vec::with_capacity(patterns.len());
        for (payload, (pattern, storage)) in patterns.iter().zip(storage).enumerate() {
            let boxed = BoxedPayload::at(owner, PayloadPosition { variant, payload });
            let payloads = self.variant_payloads();
            let inferred = payloads.is_inferred_box(boxed);
            let payload = payloads.seen_by_pattern(boxed, storage.clone());
            let mut value = self.lower_pattern(pattern, &payload, mode)?;
            value.boxed = inferred;
            typed.push(value);
        }
        Ok(typed)
    }

    fn short_pattern_target(
        &mut self,
        short: &ShortPattern<'_>,
        matched: &Type,
    ) -> Result<VariantTarget, Diagnostic> {
        if let Type::Named(owner, arguments) = self.unifier.shallow(matched) {
            let named = short
                .candidates
                .iter()
                .find(|candidate| self.variant_info(**candidate).owner == owner);
            if let Some(&binding) = named {
                return Ok(VariantTarget::new(
                    binding,
                    self.variant_info(binding),
                    arguments,
                ));
            }
            if let Some(target) = self.declared_short_target(short, owner, arguments) {
                return target;
            }
        }
        let binding = if matches!(self.unifier.shallow(matched), Type::Var(_)) {
            self.unique_candidate(short.candidates, short.span)?
        } else {
            self.candidates_by_owner(short.candidates)[0]
        };
        let info = self.variant_info(binding);
        let arguments = self.fresh_arguments(info.owner);
        let expected = Type::Named(info.owner, arguments.clone());
        self.unification()
            .expect(Expectation::plain(&expected), matched, short.span)?;
        Ok(VariantTarget::new(binding, info, arguments))
    }

    fn declared_short_target(
        &self,
        short: &ShortPattern<'_>,
        owner: BindingId,
        arguments: Vec<Type>,
    ) -> Option<Result<VariantTarget, Diagnostic>> {
        let definition = self.declarations.types().enums.get(owner)?;
        let enum_name = self.bindings.get(owner).name();
        let index = VariantNames::of(definition).index_of(short.name, enum_name, short.span);
        Some(index.map(|index| {
            let binding = self.candidate_binding(short.candidates, owner, index);
            let info = VariantInfo {
                owner,
                index,
                arity: definition.variants[index].arity(),
            };
            VariantTarget::new(binding, info, arguments)
        }))
    }

    fn candidates_by_owner(&self, candidates: &[BindingId]) -> Vec<BindingId> {
        let mut by_owner = candidates.to_vec();
        by_owner.sort_by_key(|candidate| self.variant_info(*candidate).owner);
        by_owner.dedup_by_key(|candidate| self.variant_info(*candidate).owner);
        by_owner
    }

    fn unique_candidate(
        &self,
        candidates: &[BindingId],
        span: Span,
    ) -> Result<BindingId, Diagnostic> {
        let by_owner = self.candidates_by_owner(candidates);
        match by_owner.as_slice() {
            [candidate] => Ok(*candidate),
            _ => {
                let names: Vec<String> = by_owner
                    .iter()
                    .map(|candidate| self.variant_info(*candidate).owner)
                    .map(|owner| self.bindings.get(owner).name().to_string())
                    .collect();
                Err(TypeProblems::ambiguous_variant("", &names, span))
            }
        }
    }

    fn candidate_binding(
        &self,
        candidates: &[BindingId],
        owner: BindingId,
        index: usize,
    ) -> BindingId {
        *candidates
            .iter()
            .find(|binding| {
                matches!(
                    self.bindings.get(**binding).kind(),
                    BindingKind::Variant(_, info) if info.owner == owner && info.index == index
                )
            })
            .unwrap_or(&candidates[0])
    }

    fn match_arguments(
        &mut self,
        matched: Type,
        owner: BindingId,
        span: Span,
    ) -> Result<Vec<Type>, Diagnostic> {
        match self.unifier.shallow(&matched) {
            Type::Named(binding, arguments) if binding == owner => Ok(arguments),
            Type::Var(_) => {
                let arguments = self.fresh_arguments(owner);
                let named = Type::Named(owner, arguments.clone());
                self.unification()
                    .expect(Expectation::plain(&named), &matched, span)?;
                Ok(arguments)
            }
            found => {
                let expected = Type::Named(owner, self.fresh_arguments(owner));
                self.unification()
                    .expect(Expectation::plain(&expected), &found, span)?;
                Ok(Vec::new())
            }
        }
    }

    fn binding_mode_type(matched: Type, mode: Option<ReferenceKind>) -> Type {
        match mode {
            Some(ReferenceKind::Shared) => Type::shared(matched),
            Some(ReferenceKind::Mutable) => Type::mutable(matched),
            None => matched,
        }
    }

    fn typed(
        inferred: Result<(TypedExprKind, Type), Diagnostic>,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        inferred.map(|(kind, ty)| TypedExpr { kind, ty, span })
    }

    pub(super) fn global_type(&mut self, binding: BindingId, span: Span) -> Type {
        if let Some(current) = self.component.get(&binding) {
            return current.clone();
        }
        match self.schemes.get(&binding).cloned() {
            Some(scheme) => self.instantiation().instantiate(&scheme, span),
            None => {
                assert!(
                    !self.outcomes.is_unreached(binding),
                    "the definition graph missed a reference to the prelude's `{}`",
                    self.bindings.get(binding).name()
                );
                self.unifier.fresh()
            }
        }
    }

    pub(super) fn infer_against(
        &mut self,
        expr: &Expr,
        expected: &Type,
        context: MismatchContext,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = self.infer_expecting(expr, expected, &context)?;
        let value = self.copy_out_of_borrow(value, expected);
        self.unification()
            .expect(Expectation::new(expected, context), &value.ty, value.span)?;
        Ok(value)
    }

    fn infer_expecting(
        &mut self,
        expr: &Expr,
        expected: &Type,
        context: &MismatchContext,
    ) -> Result<TypedExpr, Diagnostic> {
        let shape = self.unifier.shallow(expected);
        InferredSpan::around(self, expr.span(), |checker| match (expr.kind(), shape) {
            (ExprKind::Fn(lambda), Type::Function(_, parameters, result))
                if parameters.len() == lambda.params.len() =>
            {
                let expectation = ExpectedFunction {
                    parameters: &parameters,
                    result: &result,
                    context: context.clone(),
                    written: matches!(context, MismatchContext::Signature { .. }),
                };
                let (kind, ty) = checker.infer_lambda(lambda, expr.span(), Some(expectation))?;
                Ok(TypedExpr {
                    kind,
                    ty,
                    span: expr.span(),
                })
            }
            (ExprKind::Call(callee, arguments), _) => {
                checker.infer_call_expecting(callee, arguments, Some(expected), expr.span())
            }
            (ExprKind::Control(Control::Match(value, arms)), _) => {
                checker.infer_match_expecting(value, arms, Some(expected), expr.span())
            }
            (ExprKind::Variant(binding, arguments), _) => {
                VariantConstruction::new(*binding, arguments, expr.span())
                    .type_against(checker, Some(Expectation::plain(expected)))
            }
            (ExprKind::LabeledVariant(binding, fields), _) => {
                checker.infer_labeled_variant(*binding, fields, Some(expected), expr.span())
            }
            _ => checker.infer(expr),
        })
    }

    fn copy_out_of_borrow(&self, value: TypedExpr, expected: &Type) -> TypedExpr {
        if self.passing_facts().passes_as_written(&value) {
            return value;
        }
        let span = value.span;
        let expected = self.unifier.resolve(expected);
        let found = self.unifier.resolve(&value.ty);
        match found {
            Type::Ref(_, referent)
                if *referent == expected && self.copy_rules().is_small_copy(&expected) =>
            {
                TypedExpr {
                    kind: TypedExprKind::Deref(Box::new(value)),
                    ty: expected,
                    span,
                }
            }
            _ => value,
        }
    }

    fn infer_lambda(
        &mut self,
        lambda: &Lambda,
        span: Span,
        expected: Option<ExpectedFunction>,
    ) -> Result<(TypedExprKind, Type), Diagnostic> {
        let mut params = Vec::new();
        for (position, param) in lambda.params.iter().enumerate() {
            let expected_parameter = expected.as_ref().and_then(|expected| {
                expected
                    .parameters
                    .get(position)
                    .map(|parameter| (parameter, expected.written))
            });
            params.push(self.bind_parameter(param, expected_parameter)?);
        }
        let result = match &expected {
            Some(expected) => expected.result.clone(),
            None => self.unifier.fresh(),
        };
        let target = RecurTarget {
            parameters: params.iter().map(|param| param.ty.clone()).collect(),
            result: result.clone(),
        };
        let previous = self.recur_target.replace(target);
        let enclosing_body = std::mem::replace(&mut self.checking_function_body, true);
        let previous_try = self.try_target.replace(TryTarget {
            returns: result.clone(),
        });
        let context = expected
            .as_ref()
            .map_or(MismatchContext::Plain, |expected| expected.context.clone());
        let inferred = InferredSpan::around(self, span, |checker| {
            let tail = Expectation::new(&result, context.clone());
            Body::new(&lambda.body).type_with(checker, Some(tail))
        });
        self.recur_target = previous;
        self.checking_function_body = enclosing_body;
        self.try_target = previous_try;
        let (body, found) = inferred?;
        let result_span = ResultSpan::of_block(&body).unwrap_or(span);
        self.unification()
            .expect(Expectation::new(&result, context), &found, result_span)?;
        let body = self.view_closure_result(body, &result, &found)?;
        let parameter_types = params.iter().map(|param| param.ty.clone()).collect();
        let function = Type::Function(
            crate::types::ClosureKind::Fn,
            parameter_types,
            Box::new(result),
        );
        Ok((
            TypedExprKind::Fn(Rc::new(TypedLambda {
                params,
                body,
                wraps: None,
                kind: crate::types::ClosureKind::Fn,
                facts: crate::types::ClosureFacts::default(),
            })),
            function,
        ))
    }

    fn bind_parameter(
        &mut self,
        param: &Binder,
        expected: Option<(&Type, bool)>,
    ) -> Result<TypedBinder, Diagnostic> {
        let ty = match (&param.annotation, expected) {
            (Some(annotation), _) => self.written_types().lower_without_generics(annotation)?,
            (None, Some((expected, _))) => expected.clone(),
            (None, None) => self.unifier.fresh(),
        };
        self.locals.insert(param.binding, ty.clone());
        if self.inferred_copies.contains(param.binding) {
            self.require_copy(&ty);
        }
        let written_by_signature = expected.is_some_and(|(expected, written)| {
            written && !matches!(self.unifier.shallow(expected), Type::Var(_))
        });
        let signature = written_by_signature.then_some(&ty);
        Ok(TypedBinder {
            binding: param.binding,
            written_view: TypedBinder::written_view(param.annotation.as_ref(), signature),
            ty,
            span: param.span,
            annotated: param.annotation.is_some() || written_by_signature,
        })
    }

    fn infer_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        self.infer_call_expecting(callee, arguments, None, span)
    }

    fn infer_call_expecting(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let callee = self.infer(callee)?;
        let (parameters, result) = self.function_shape(&callee, arguments.len())?;
        if parameters.len() != arguments.len() {
            return Err(TypeProblems::wrong_argument_count(
                parameters.len(),
                arguments.len(),
                span,
            ));
        }
        let boxed = self.box_object(expected, &result);
        if boxed.is_none() {
            self.expect_call_result(expected, &result, span)?;
        }
        let typed_arguments = self.type_arguments(Some(&callee), &parameters, arguments)?;
        let call = TypedExpr {
            kind: TypedExprKind::Call(Box::new(callee), typed_arguments),
            ty: result,
            span,
        };
        self.finish_call(call, boxed, expected, span)
    }

    fn finish_call(
        &mut self,
        call: TypedExpr,
        boxed: Option<BindingId>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        match (boxed, expected) {
            (Some(trait_binding), Some(expected)) => {
                self.coerce_box(call, trait_binding, expected, span)
            }
            _ => Ok(call),
        }
    }

    fn box_object(&self, expected: Option<&Type>, result: &Type) -> Option<BindingId> {
        let expected = expected?;
        let Type::Named(expected_box, expected_arguments) = self.unifier.resolve(expected) else {
            return None;
        };
        let [Type::TraitObject(trait_binding)] = expected_arguments.as_slice() else {
            return None;
        };
        let Type::Named(result_box, result_arguments) = self.unifier.resolve(result) else {
            return None;
        };
        if result_box != expected_box || result_arguments.len() != 1 {
            return None;
        }
        Some(*trait_binding)
    }

    fn coerce_box(
        &mut self,
        call: TypedExpr,
        trait_binding: BindingId,
        expected: &Type,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let Type::Named(_, arguments) = self.unifier.resolve(&call.ty) else {
            unreachable!("a boxed call has a box type")
        };
        let argument = arguments
            .first()
            .expect("a boxed call has one type argument");
        if matches!(argument, Type::TraitObject(object) if *object == trait_binding) {
            return Ok(TypedExpr {
                ty: expected.clone(),
                ..call
            });
        }
        let bound = Bound::plain(trait_binding);
        let satisfied = self
            .declarations
            .traits()
            .table
            .satisfies(&bound, argument, &|_, _| false);
        if !satisfied {
            self.report_missing_trait(&bound, argument, span);
        }
        Ok(TypedExpr {
            kind: TypedExprKind::TraitObject(Box::new(call), trait_binding),
            ty: expected.clone(),
            span,
        })
    }

    fn expect_call_result(
        &mut self,
        expected: Option<&Type>,
        result: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Some(expected) = expected else {
            return Ok(());
        };
        let resolved = self.unifier.resolve(result);
        let expected_resolved = self.unifier.resolve(expected);
        if resolved.has_vars()
            && (resolved.shares_head_with(&expected_resolved)
                || self
                    .unification()
                    .coerces(&expected_resolved, &resolved, span))
        {
            self.unification()
                .expect(Expectation::plain(expected), result, span)?;
        }
        Ok(())
    }

    fn infer_try(&mut self, value: &Expr, span: Span) -> Result<TypedExpr, Diagnostic> {
        let target = self
            .try_target
            .clone()
            .expect("expansion checks that `try` is inside a function");
        let value = self.infer(value)?;
        let operand = self.unifier.resolve(&value.ty);
        let Some(source) = self.try_source(&operand) else {
            if operand.is_unknown()
                && let Some(reported) = self.report_unknown_recursive_try(&value, span)
            {
                return reported;
            }
            return Err(TypeProblems::try_needs_value(
                &self.unification().describe(&operand),
                span,
            ));
        };
        let expected = self.try_return_type(&source);
        let conversion = if self
            .unifier
            .unify(&expected, &target.returns, span)
            .is_err()
        {
            self.convert_try_error(&source, &target.returns, span)?;
            TryErrorConversion::Converted
        } else {
            TryErrorConversion::Identity
        };
        Ok(TypedExpr {
            kind: TypedExprKind::Try {
                value: Box::new(value),
                conversion,
            },
            ty: source.value,
            span,
        })
    }

    fn report_unknown_recursive_try(
        &mut self,
        value: &TypedExpr,
        span: Span,
    ) -> Option<Result<TypedExpr, Diagnostic>> {
        let called = self.recursive_try_callee(value)?;
        let group = self.recursive_group.iter().copied().min()?;
        if !self.reported_unknown_try_groups.insert(group) {
            return Some(Ok(TypedExpr {
                kind: TypedExprKind::Try {
                    value: Box::new(value.clone()),
                    conversion: TryErrorConversion::Identity,
                },
                ty: self.unifier.fresh(),
                span,
            }));
        }
        let called_name = self.bindings.get(called).name();
        let names: Vec<String> = self
            .recursive_group
            .iter()
            .map(|binding| self.bindings.get(*binding).name().to_string())
            .collect();
        let facts = &self.checking_facts;
        let hides_prelude = facts.prelude_defines_value(called_name, &self.bindings);
        Some(Err(TypeProblems::try_needs_unknown_recursive(
            called_name,
            &names,
            hides_prelude,
            span,
        )))
    }

    fn recursive_try_callee(&self, value: &TypedExpr) -> Option<BindingId> {
        let TypedExprKind::Call(callee, _) = &value.kind else {
            return None;
        };
        let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = &callee.kind else {
            return None;
        };
        self.recursive_group.contains(binding).then_some(*binding)
    }

    fn convert_try_error(
        &mut self,
        source: &TrySource,
        returns: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let returns = self.unifier.resolve(returns);
        let Some(target) = self.try_source(&returns) else {
            return Err(self.try_target_problem(source, &returns, span));
        };
        if source.kind != TrySourceKind::Result || target.kind != TrySourceKind::Result {
            return Err(self.try_target_problem(source, &returns, span));
        }
        let source_error = source
            .error
            .clone()
            .expect("a result source carries an error type");
        let target_error = target.error.expect("a result target carries an error type");
        let conversion = self.conversion_variant(&target_error, &source_error, span)?;
        self.record_conversion(conversion);
        Ok(())
    }

    fn try_target_problem(&mut self, source: &TrySource, returns: &Type, span: Span) -> Diagnostic {
        let source_name = self.bindings.get(source.binding).name();
        TypeProblems::try_cannot_return(
            &source.kind.needed(source_name),
            &self.unification().describe(returns),
            span,
        )
    }

    fn conversion_variant(
        &mut self,
        error: &Type,
        source: &Type,
        span: Span,
    ) -> Result<TryConversion, Diagnostic> {
        let Type::Named(binding, _) = error else {
            return Err(self.no_conversion(error, source, span));
        };
        let Some(definition) = self.declarations.types().enums.get(*binding) else {
            return Err(self.no_conversion(error, source, span));
        };
        if definition.rust.is_some() || !definition.params.is_empty() {
            return Err(self.no_conversion(error, source, span));
        }
        let candidates: Vec<(usize, Rc<str>, Type)> = definition
            .variants
            .iter()
            .enumerate()
            .filter_map(|(variant, candidate)| {
                let VariantPayload::Positional(payloads) = &candidate.payload else {
                    return None;
                };
                let [payload] = payloads.as_slice() else {
                    return None;
                };
                (!self.declarations.types().is_box(payload) && Self::could_unify(payload, source))
                    .then(|| (variant, Rc::clone(&candidate.name), payload.clone()))
            })
            .collect();
        if candidates.len() > 1 {
            let variants: Vec<(String, String)> = candidates
                .iter()
                .map(|(_, name, payload)| (name.to_string(), self.unification().describe(payload)))
                .collect();
            return Err(TypeProblems::try_ambiguous_conversion(
                &self.unification().describe(source),
                &self.unification().describe(error),
                &variants,
                span,
            ));
        }
        let Some((variant, _, payload)) = candidates.first() else {
            return Err(self.no_conversion(error, source, span));
        };
        self.unifier
            .unify(payload, source, span)
            .map_err(|_| self.no_conversion(error, source, span))?;
        Ok(TryConversion {
            enum_binding: *binding,
            variant: *variant,
            payload: payload.clone(),
        })
    }

    fn no_conversion(&mut self, error: &Type, source: &Type, span: Span) -> Diagnostic {
        TypeProblems::try_no_conversion(
            &self.unification().describe(source),
            &self.unification().describe(error),
            span,
        )
    }

    fn could_unify(left: &Type, right: &Type) -> bool {
        if !right.has_vars() {
            return left == right;
        }
        match (left, right) {
            (_, Type::Var(_)) => true,
            (Type::Named(left, left_parts), Type::Named(right, right_parts)) => {
                left == right
                    && left_parts.len() == right_parts.len()
                    && left_parts
                        .iter()
                        .zip(right_parts)
                        .all(|(left, right)| Self::could_unify(left, right))
            }
            _ => false,
        }
    }

    fn record_conversion(&mut self, conversion: TryConversion) {
        let known = self.deferred.conversions.iter().any(|known| {
            known.enum_binding == conversion.enum_binding && known.error == conversion.payload
        });
        if !known {
            self.deferred.conversions.push(TypedConversion {
                enum_binding: conversion.enum_binding,
                variant: conversion.variant,
                error: conversion.payload,
            });
        }
    }

    fn try_source(&self, operand: &Type) -> Option<TrySource> {
        let Type::Named(binding, arguments) = operand else {
            return None;
        };
        let definition = self.declarations.types().enums.get(*binding)?;
        if definition.is_result_shaped() && arguments.len() == 2 {
            return Some(TrySource {
                kind: TrySourceKind::Result,
                binding: *binding,
                value: arguments[0].clone(),
                error: Some(arguments[1].clone()),
            });
        }
        if definition.is_option_shaped() && arguments.len() == 1 {
            return Some(TrySource {
                kind: TrySourceKind::Option,
                binding: *binding,
                value: arguments[0].clone(),
                error: None,
            });
        }
        None
    }

    fn try_return_type(&mut self, source: &TrySource) -> Type {
        let value = self.unifier.fresh();
        match source.kind {
            TrySourceKind::Result => Type::Named(
                source.binding,
                vec![
                    value,
                    source
                        .error
                        .clone()
                        .expect("a result source carries an error type"),
                ],
            ),
            TrySourceKind::Option => Type::Named(source.binding, vec![value]),
        }
    }

    fn infer_arguments(
        &mut self,
        callee: Option<&TypedExpr>,
        parameters: &[Type],
        arguments: &[Expr],
        shared: &SharedVariables,
    ) -> Result<Vec<TypedArgument>, Diagnostic> {
        let mut typed_arguments = Vec::new();
        for (position, (parameter, argument)) in parameters.iter().zip(arguments).enumerate() {
            let site = ArgumentSite { callee, position };
            let value = self.infer_argument(parameter, argument, &site)?;
            let borrow = if shared.contains(position) {
                Borrow::None
            } else {
                self.pass_call_argument(callee, position, parameter, &value)?
            };
            typed_arguments.push(TypedArgument { value, borrow });
        }
        Ok(typed_arguments)
    }

    fn infer_argument(
        &mut self,
        parameter: &Type,
        argument: &Expr,
        site: &ArgumentSite<'_>,
    ) -> Result<TypedExpr, Diagnostic> {
        let span = argument.span();
        let ExprKind::Fn(lambda) = argument.kind() else {
            let expected = parameter.clone().without_reference();
            let value = if self.contains_box_object(&expected) {
                self.infer_expecting(argument, &expected, &MismatchContext::Plain)?
            } else {
                self.infer(argument)?
            };
            let value = self.wrap_function_value(parameter, value)?;
            return self.iterate_collection(parameter, value, site);
        };
        match self.unifier.shallow(parameter) {
            Type::Function(_, parameters, result) if parameters.len() == lambda.params.len() => {
                let expectation = ExpectedFunction {
                    parameters: &parameters,
                    result: &result,
                    context: MismatchContext::Plain,
                    written: true,
                };
                let (kind, ty) = self.infer_lambda(lambda, span, Some(expectation))?;
                Ok(TypedExpr { kind, ty, span })
            }
            _ => self.infer(argument),
        }
    }

    fn iterate_collection(
        &mut self,
        parameter: &Type,
        value: TypedExpr,
        site: &ArgumentSite<'_>,
    ) -> Result<TypedExpr, Diagnostic> {
        let expected = self.unifier.resolve(parameter);
        if value.is_written_form() || !self.prelude_types().is_pipeline(&expected) {
            return Ok(value);
        }
        let Some((item, collection_type)) = self.collection_item(&value.ty) else {
            return Ok(value);
        };
        let (template, item) = self.iteration(&expected, item);
        let result = self.prelude_types().iterator_type(item);
        let parameters = vec![Type::shared(collection_type)];
        let span = value.span;
        let call =
            self.synthetic_call_with_value(template, parameters, result, value, &[], span)?;
        self.unification()
            .expect(Expectation::plain(&expected), &call.ty, span)
            .map_err(|diagnostic| self.explain_borrowed_items(site, &call.ty, diagnostic))?;
        Ok(call)
    }

    fn iteration(&self, expected: &Type, item: Type) -> (&'static str, Type) {
        match item {
            Type::Ref(_, element) if self.expects_copied_items(expected, &element) => {
                (SyntheticTemplate::ITERATE_COPIED.text(), *element)
            }
            item => (SyntheticTemplate::ITERATE.text(), item),
        }
    }

    fn expects_copied_items(&self, expected: &Type, element: &Type) -> bool {
        let Type::Named(_, arguments) = expected else {
            return false;
        };
        let [item] = arguments.as_slice() else {
            return false;
        };
        match self.unifier.shallow(item) {
            Type::Var(var) => self.passing_facts().allows_only_copy_values(var),
            item => self.is_copy_element(&item, element),
        }
    }

    fn is_copy_element(&self, item: &Type, element: &Type) -> bool {
        let item = self.unifier.resolve(item).without_written_lifetimes();
        self.copy_rules().is_copy(&item) && self.element_is(element, &item)
    }

    fn element_is(&self, element: &Type, item: &Type) -> bool {
        match (self.unifier.shallow(element), item) {
            (Type::Var(var), Type::Primitive(primitive)) => self
                .unifier
                .literal_kind(var)
                .is_some_and(|kind| kind.accepts(*primitive)),
            (element, item) => &self.unifier.resolve(&element).without_written_lifetimes() == item,
        }
    }

    fn collection_item(&self, ty: &Type) -> Option<(Type, Type)> {
        let resolved = self.unifier.resolve(ty);
        let collection_type = match &resolved {
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.unifier.resolve(inner),
            _ => resolved.clone(),
        };
        if let Type::Named(binding, arguments) = &collection_type
            && self.declarations.types().rust_types.is_slice(*binding)
        {
            let [element] = arguments.as_slice() else {
                return None;
            };
            return Some((
                Type::shared(self.unifier.resolve(element)),
                collection_type.clone(),
            ));
        }
        let collection = Collection::of(&collection_type, &self.declarations.types().rust_types)?;
        let item = if collection.is_map() {
            let key = self.unifier.resolve(collection.key());
            let entry = self.unifier.resolve(collection.value());
            self.prelude_types()
                .pair_of(Type::shared(key), Type::shared(entry))
        } else {
            Type::shared(self.unifier.resolve(collection.value()))
        };
        Some((item, collection.ty().clone()))
    }

    fn contains_box_object(&self, ty: &Type) -> bool {
        match self.unifier.resolve(ty) {
            Type::TraitObject(_) => true,
            resolved if self.declarations.types().is_box(&resolved) => {
                self.declarations.types().boxed_object(&resolved).is_some()
            }
            Type::Named(_, arguments) => arguments
                .iter()
                .any(|argument| self.contains_box_object(argument)),
            Type::Ref(_, inner) | Type::Mut(_, inner) => self.contains_box_object(&inner),
            Type::Function(_, parameters, result) => {
                self.function_contains_box_object(&parameters, &result)
            }
            Type::Var(_) | Type::Generic(_) | Type::Primitive(_) => false,
        }
    }

    fn function_contains_box_object(&self, parameters: &[Type], result: &Type) -> bool {
        parameters
            .iter()
            .any(|parameter| self.contains_box_object(parameter))
            || self.contains_box_object(result)
    }

    fn wrap_function_value(
        &mut self,
        parameter: &Type,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = &value.kind else {
            return Ok(value);
        };
        let binding = *binding;
        let (Type::Function(_, expected, expected_result), Type::Function(_, found, found_result)) = (
            self.unifier.resolve(parameter),
            self.unifier.resolve(&value.ty),
        ) else {
            return Ok(value);
        };
        if expected.len() != found.len() || !Self::needs_wrapper(&expected, &found) {
            return Ok(value);
        }
        self.give_items_to_parameters(binding, &expected, &found, value.span)?;
        let mut wrapped = self
            .function_value_wrapper()
            .prepare(&expected, value, binding);
        for argument in &mut wrapped.arguments {
            argument.borrow = Borrow::None;
        }
        for param in &wrapped.params {
            self.locals.insert(param.binding, param.ty.clone());
        }
        self.unification().expect(
            Expectation::plain(expected_result.as_ref()),
            found_result.as_ref(),
            wrapped.span(),
        )?;
        Ok(wrapped.finish(&expected_result, &found_result))
    }

    fn give_items_to_parameters(
        &mut self,
        function: BindingId,
        expected: &[Type],
        found: &[Type],
        span: Span,
    ) -> Result<(), Diagnostic> {
        if self.outcomes.follows_reported_error(function) {
            return Ok(());
        }
        for (position, (expected, found)) in expected.iter().zip(found).enumerate() {
            if !found.is_reference() {
                let (item, _) = self.reference_peeling().peel(expected);
                self.unification()
                    .expect(Expectation::plain(&item), found, span)
                    .map_err(|diagnostic| {
                        self.explain_literal_default(function, position, diagnostic)
                    })?;
            }
        }
        Ok(())
    }

    fn needs_wrapper(expected: &[Type], found: &[Type]) -> bool {
        expected
            .iter()
            .zip(found)
            .any(|(expected, found)| expected.is_reference() && !found.is_reference())
    }

    fn function_shape(
        &mut self,
        callee: &TypedExpr,
        argument_count: usize,
    ) -> Result<(Vec<Type>, Type), Diagnostic> {
        match self.unifier.shallow(&callee.ty) {
            Type::Function(_, parameters, result) => Ok((parameters, *result)),
            Type::Var(var) if self.unifier.literal_kind(var).is_none() => {
                let parameters: Vec<Type> =
                    (0..argument_count).map(|_| self.unifier.fresh()).collect();
                let result = self.unifier.fresh();
                let shape = Type::Function(
                    crate::types::ClosureKind::Fn,
                    parameters.clone(),
                    Box::new(result.clone()),
                );
                self.unification()
                    .expect(Expectation::plain(&shape), &callee.ty, callee.span)?;
                Ok((parameters, result))
            }
            other => Err(TypeProblems::not_a_function(
                &self.unification().describe(&other),
                callee.span,
            )),
        }
    }

    pub(super) fn pass_call_argument(
        &mut self,
        callee: Option<&TypedExpr>,
        position: usize,
        parameter: &Type,
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic> {
        if let Type::Ref(_, referent) = self.unifier.shallow(parameter)
            && value.kind.is_dereference()
            && callee.is_some_and(|callee| self.template_borrows.is_template(callee))
            && self.passing_facts().passes_as_written(value)
        {
            return self
                .unification()
                .expect(Expectation::plain(&referent), &value.ty, value.span)
                .map(|()| Borrow::Shared);
        }
        if let Some(passed) = (self.argument_passing()).pass_held_view(callee, parameter, value) {
            return passed;
        }
        self.pass_to_callee(callee, position, parameter, value)
    }

    fn pass_to_callee(
        &mut self,
        callee: Option<&TypedExpr>,
        position: usize,
        parameter: &Type,
        value: &TypedExpr,
    ) -> Result<Borrow, Diagnostic> {
        let call = match callee.map(|callee| &callee.kind) {
            Some(TypedExprKind::Leaf(TypedLeaf::Global(binding))) => *binding,
            _ => return self.argument_passing().pass_argument(parameter, value),
        };
        if let TypedExprKind::Borrow(kind, _) = &value.kind
            && !self.checking_facts.written_at(call, position)
            && !self.unifier.shallow(parameter).is_reference()
            && let Type::Ref(_, referent) | Type::Mut(_, referent) = &value.ty
        {
            self.unification()
                .expect(Expectation::plain(parameter), referent, value.span)
                .map_err(|diagnostic| {
                    self.explain_assumed_collection(call, position, &value.ty, diagnostic)
                })?;
            self.deferred.mode_checks.push(PendingModeCheck {
                callee: call,
                position,
                kind: *kind,
                span: value.span,
            });
            return Ok(Borrow::None);
        }
        let site = ArgumentSite { callee, position };
        self.argument_passing()
            .pass_argument(parameter, value)
            .map_err(|diagnostic| self.explain_literal_default(call, position, diagnostic))
            .inspect_err(|diagnostic| {
                self.request_parameter_call_label(call, position, &value.ty, diagnostic)
            })
            .map_err(|diagnostic| self.explain_borrowed_items(&site, &value.ty, diagnostic))
            .map_err(|diagnostic| {
                self.explain_assumed_collection(call, position, &value.ty, diagnostic)
            })
    }

    fn explain_assumed_collection(
        &self,
        callee: BindingId,
        position: usize,
        found: &Type,
        diagnostic: Diagnostic,
    ) -> Diagnostic {
        let Some(assumption) = self.assumed_collections.get(callee, position) else {
            return diagnostic;
        };
        let function = self.bindings.get(callee).name();
        let signature = self.signature_with_parameter(callee, position, found);
        diagnostic.with_help(AssumedCollectionProblems::help(
            assumption, function, &signature,
        ))
    }

    fn signature_with_parameter(&self, callee: BindingId, position: usize, found: &Type) -> String {
        let count = self.checking_facts.written_parameter_count(callee);
        let shown = self.display().show(&self.unifier.resolve(found));
        let parameters: Vec<&str> = (0..count)
            .map(|index| {
                if index == position {
                    shown.as_str()
                } else {
                    "_"
                }
            })
            .collect();
        format!("(fn ({}) _)", parameters.join(" "))
    }

    fn explain_literal_default(
        &self,
        callee: BindingId,
        position: usize,
        diagnostic: Diagnostic,
    ) -> Diagnostic {
        if diagnostic.code() != Some(&DiagnosticCode::MISMATCHED_TYPES) {
            return diagnostic;
        }
        let Some(default) = self.literal_defaults.get(callee, position) else {
            return diagnostic;
        };
        let function = self.bindings.get(callee).name();
        diagnostic
            .with_note(TypeProblems::literal_default_note(default, function))
            .with_help(TypeProblems::literal_default_help(default, function))
    }

    fn request_parameter_call_label(
        &mut self,
        callee: BindingId,
        position: usize,
        found: &Type,
        diagnostic: &Diagnostic,
    ) {
        if self.function_type_comes_from_call(callee, position, found, diagnostic) {
            self.deferred.label_requests.push(ParameterCallRequest {
                callee,
                position,
                argument: diagnostic.primary().span(),
            });
        }
    }

    fn function_type_comes_from_call(
        &self,
        function: BindingId,
        position: usize,
        found: &Type,
        diagnostic: &Diagnostic,
    ) -> bool {
        diagnostic.code() == Some(&DiagnosticCode::MISMATCHED_TYPES)
            && !self.checking_facts.written_at(function, position)
            && !matches!(self.unifier.shallow(found), Type::Function(..))
    }

    pub(super) fn decide_pending_borrows(&mut self) -> DecidedBorrows {
        let pending = std::mem::take(&mut self.deferred.borrows);
        let mut decided = FxHashMap::default();
        for (id, pending) in pending.iter().enumerate() {
            let decision = self.argument_passing().decide(pending);
            let borrow = decision.unwrap_or_else(|diagnostic| {
                self.diagnostics.push(diagnostic);
                Borrow::None
            });
            decided.insert(id, borrow);
        }
        DecidedBorrows::new(decided)
    }

    fn infer_new(
        &mut self,
        binding: BindingId,
        fields: &[(FieldLabel, Expr)],
        span: Span,
    ) -> Result<(TypedExprKind, Type), Diagnostic> {
        let struct_name = self.bindings.get(binding).name().to_string();
        let arguments = self.fresh_arguments(binding);
        let typed_fields = self.check_fields(binding, &arguments, fields)?;
        let declared = self.instantiated_fields(binding, &arguments);
        Self::require_every_field(&struct_name, &declared, fields, span)?;
        Ok((
            TypedExprKind::Struct(TypedStructExpr::New(binding, typed_fields)),
            Type::Named(binding, arguments),
        ))
    }

    fn infer_map_at(
        &mut self,
        operation: MapOperation,
        value: &Expr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = self.infer(value)?;
        match operation {
            MapOperation::Iter => self.infer_iter(value, arguments, span),
            MapOperation::Get => self.infer_get(value, arguments, span),
            MapOperation::First => self.infer_part(PositionalRead::first(span), value, arguments),
            MapOperation::Second => self.infer_part(PositionalRead::second(span), value, arguments),
            _ => self.infer_map_only(operation, value, arguments, span),
        }
    }

    fn infer_map_only(
        &mut self,
        operation: MapOperation,
        value: TypedExpr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let collection = self.map_collection(&value)?;
        self.check_map_key(&collection, value.span);
        match operation {
            MapOperation::Update => self.infer_map_update(value, collection, arguments, span),
            MapOperation::Keys => {
                self.infer_map_iterator(value, collection, MapIterator::Keys, arguments, span)
            }
            _ => self.infer_map_iterator(value, collection, MapIterator::Vals, arguments, span),
        }
    }

    fn infer_iter(
        &mut self,
        value: TypedExpr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        if !arguments.is_empty() {
            return Err(TypeProblems::wrong_argument_count(0, arguments.len(), span));
        }
        let Some((item, collection_type)) = self.collection_item(&value.ty) else {
            let resolved = self.unifier.resolve(&value.ty);
            if resolved.is_unknown() && !self.is_literal(&resolved) {
                return self.infer_iter_slice(value, span);
            }
            return Err(TypeProblems::iter_needs_collection(
                &self.unification().describe(&resolved),
                value.span,
            ));
        };
        if let Some(collection) =
            Collection::of(&collection_type, &self.declarations.types().rust_types)
        {
            if collection.kind() == CollectionKind::Vector {
                return self.infer_iter_slice(value, span);
            }
            if collection.is_map() {
                self.check_map_key(&collection, value.span);
            }
        }
        let result = self.prelude_types().iterator_type(item);
        let parameters = vec![Type::shared(collection_type)];
        let template = SyntheticTemplate::ITERATE.text();
        self.synthetic_call_with_value(template, parameters, result, value, arguments, span)
    }

    fn infer_get(
        &mut self,
        value: TypedExpr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let resolved = self.unifier.resolve(&value.ty);
        match Collection::of_referent(&resolved, &self.declarations.types().rust_types)
            .filter(Collection::is_map)
        {
            Some(collection) => {
                self.check_map_key(&collection, value.span);
                self.infer_map_lookup(value, collection, arguments, span)
            }
            None => self.infer_slice_get(value, arguments, span),
        }
    }

    fn infer_slice_get(
        &mut self,
        value: TypedExpr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        if arguments.len() != 1 {
            return Err(TypeProblems::wrong_argument_count(1, arguments.len(), span));
        }
        self.note_unknown_collection(&value, CollectionAssumption::GetCollection);
        let element = self.unifier.fresh();
        let parameters = vec![
            Type::shared(self.prelude_types().slice_of(element.clone())),
            Collection::INDEX,
        ];
        let result = self.prelude_types().option_type(element);
        let template = SyntheticTemplate::SLICE_LOOK_UP.text();
        self.synthetic_call_with_value(template, parameters, result, value, arguments, span)
    }

    fn infer_part(
        &mut self,
        read: PositionalRead,
        value: TypedExpr,
        arguments: &[Expr],
    ) -> Result<TypedExpr, Diagnostic> {
        let (reader, span) = (read.reader(arguments)?, read.span());
        let types = &self.declarations.types().rust_types;
        if let Some(found) = read.pair_part(&self.unifier.resolve(&value.ty), types) {
            return Ok(found.read(value, |part| self.passing_facts().holds_copy_value(part)));
        }
        self.note_unknown_collection(&value, read.slice_assumption());
        let callee = self.infer(reader)?;
        let (parameters, ty) = self.function_shape(&callee, 1)?;
        let borrow = self.pass_call_argument(Some(&callee), 0, &parameters[0], &value)?;
        let kind = TypedExprKind::Call(Box::new(callee), vec![TypedArgument { value, borrow }]);
        Ok(TypedExpr { kind, ty, span })
    }

    fn infer_push(&mut self, push: &Push, span: Span) -> Result<TypedExpr, Diagnostic> {
        let Push { target, item } = push;
        let value = self.infer(target)?;
        if StringPush::targets(self, &value.ty) {
            let item = self.infer(item)?;
            return StringPush::new(span).type_with(self, value, item);
        }
        VectorPush::new(item, span).type_with(self, value)
    }

    fn note_unknown_collection(&mut self, target: &TypedExpr, assumption: CollectionAssumption) {
        let referent = self.unifier.resolve(&target.ty).without_reference();
        if matches!(referent, Type::Var(_)) {
            self.assumed_collections.assume(target, assumption);
        }
    }

    fn infer_iter_slice(&mut self, value: TypedExpr, span: Span) -> Result<TypedExpr, Diagnostic> {
        let (template, element) = (SyntheticTemplate::ITERATE.text(), self.unifier.fresh());
        let parameter = Type::shared(self.prelude_types().slice_of(element.clone()));
        let result = self.prelude_types().iterator_type(Type::shared(element));
        self.synthetic_call_with_value(template, vec![parameter], result, value, &[], span)
    }

    fn map_collection(&self, value: &TypedExpr) -> Result<Collection, Diagnostic> {
        let resolved = self.unifier.resolve(&value.ty);
        Collection::of_referent(&resolved, &self.declarations.types().rust_types)
            .filter(Collection::is_map)
            .ok_or_else(|| {
                TypeProblems::mismatch(
                    "a map",
                    &self.display().show(&resolved),
                    value.span,
                    MismatchContext::Plain,
                )
            })
    }

    fn check_map_key(&mut self, collection: &Collection, span: Span) {
        let paths: &[&str] = if collection.kind() == CollectionKind::SortedMap {
            &[OrderTrait::PATH]
        } else {
            &[HashTrait::PATH, EqualityTrait::PATH]
        };
        let key = self.unifier.resolve(collection.key());
        if key.is_unknown() {
            return;
        }
        for path in paths {
            let binding = self
                .declarations
                .traits()
                .table
                .with_paths(&[path])
                .first()
                .copied()
                .expect("the prelude defines the map key traits");
            let bound = Bound::plain(binding);
            if !self
                .declarations
                .traits()
                .table
                .satisfies(&bound, &key, &|_, _| false)
            {
                self.report_missing_trait(&bound, &key, span);
            }
        }
    }

    fn infer_map_lookup(
        &mut self,
        value: TypedExpr,
        collection: Collection,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        if arguments.len() != 1 {
            return Err(TypeProblems::wrong_argument_count(1, arguments.len(), span));
        }
        let query_type = self.unifier.fresh();
        let parameters = vec![
            Type::shared(collection.ty().clone()),
            Type::shared(query_type),
        ];
        let result = self.prelude_types().option_type(collection.value().clone());
        let call = self.synthetic_call_with_value(
            SyntheticTemplate::LOOK_UP.text(),
            parameters,
            result,
            value,
            arguments,
            span,
        )?;
        let query = match &call.kind {
            TypedExprKind::Call(_, arguments) => {
                arguments.get(1).map(|argument| argument.value.ty.clone())
            }
            _ => None,
        };
        if let Some(query) = query {
            self.check_borrow(&collection, &query, arguments[0].span());
        }
        Ok(call)
    }

    fn check_borrow(&mut self, collection: &Collection, query: &Type, span: Span) {
        let key = self.unifier.resolve(collection.key());
        let query = self.unifier.resolve(query).without_reference();
        if key.is_unknown() || query.is_unknown() {
            return;
        }
        let binding = self
            .declarations
            .traits()
            .table
            .with_paths(&[BorrowTrait::PATH])
            .first()
            .copied()
            .expect("the prelude defines borrow");
        let bound = Bound::with_argument(binding, query);
        if !self
            .declarations
            .traits()
            .table
            .satisfies(&bound, &key, &|_, _| false)
        {
            self.report_missing_trait(&bound, &key, span);
        }
    }

    fn infer_map_iterator(
        &mut self,
        value: TypedExpr,
        collection: Collection,
        iterator: MapIterator,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        if !arguments.is_empty() {
            return Err(TypeProblems::wrong_argument_count(0, arguments.len(), span));
        }
        let (item, template) = match iterator {
            MapIterator::Keys => (collection.key().clone(), SyntheticTemplate::KEYS.text()),
            MapIterator::Vals => (collection.value().clone(), SyntheticTemplate::VALUES.text()),
        };
        let result = self.prelude_types().iterator_type(Type::shared(item));
        let parameters = vec![Type::shared(collection.ty().clone())];
        self.synthetic_call_with_value(template, parameters, result, value, arguments, span)
    }

    fn infer_map_update(
        &mut self,
        value: TypedExpr,
        collection: Collection,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        if arguments.len() != 3 {
            return Err(TypeProblems::wrong_argument_count(3, arguments.len(), span));
        }
        let value_type = collection.value().clone();
        let function_type = Type::Function(
            crate::types::ClosureKind::Fn,
            vec![value_type.clone()],
            Box::new(value_type.clone()),
        );
        let parameters = vec![
            collection.ty().clone(),
            collection.key().clone(),
            function_type,
            value_type,
        ];
        let result = collection.ty().clone();
        let template = SyntheticTemplate::MAP_UPDATE.text();
        self.synthetic_call_with_value(template, parameters, result, value, arguments, span)
    }

    fn infer_set_at(&mut self, path: &KeyedPath, span: Span) -> Result<TypedExpr, Diagnostic> {
        let KeyedPath { value, keys, item } = path;
        let value = self.infer(value)?;
        if self.is_open_collection(&value, keys.len()) {
            return self.infer_open_set_at(value, keys, item, span);
        }
        let collection = WriteCollection::new(&value, keys.len()).resolve(self)?;
        if collection.is_map() {
            return MapWrite::new(path, span).type_with(self, value, collection);
        }
        VectorWrite::new(path, span).type_with(self, value, collection)
    }

    fn synthetic_call_with_value(
        &mut self,
        template: &str,
        parameters: Vec<Type>,
        result: Type,
        value: TypedExpr,
        arguments: &[Expr],
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let mut typed_arguments = Vec::with_capacity(parameters.len());
        let borrow = self.pass_call_argument(None, 0, &parameters[0], &value)?;
        typed_arguments.push(TypedArgument { value, borrow });
        for (position, argument) in arguments.iter().enumerate() {
            let index = position + 1;
            let parameter = &parameters[index];
            let site = ArgumentSite {
                callee: None,
                position: index,
            };
            let value = self.infer_argument(parameter, argument, &site)?;
            let borrow = self.pass_call_argument(None, index, parameter, &value)?;
            typed_arguments.push(TypedArgument { value, borrow });
        }
        let signature = SyntheticSignature {
            template,
            parameters,
            result,
        };
        Ok(self.finish_synthetic_call(signature, typed_arguments, span))
    }

    fn finish_synthetic_call(
        &mut self,
        signature: SyntheticSignature<'_>,
        typed_arguments: Vec<TypedArgument>,
        span: Span,
    ) -> TypedExpr {
        let parameters: Vec<Type> = signature
            .parameters
            .iter()
            .map(|parameter| self.unifier.resolve(parameter))
            .collect();
        let result = self.unifier.resolve(&signature.result);
        let callee = self.rust_template(signature.template, parameters, result.clone(), span);
        TypedExpr {
            kind: TypedExprKind::Call(Box::new(callee), typed_arguments),
            ty: result,
            span,
        }
    }

    fn rust_template(
        &self,
        template: &str,
        parameters: Vec<Type>,
        result: Type,
        span: Span,
    ) -> TypedExpr {
        let template = RustTemplate {
            template: Rc::from(template),
            type_expr: TypeExpr::new(TypeExprKind::Hole, span),
            clauses: Vec::new(),
        };
        TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Rust(Rc::new(template))),
            ty: Type::Function(crate::types::ClosureKind::Fn, parameters, Box::new(result)),
            span,
        }
    }

    fn point_struct_base_to_fields(&self, error: Diagnostic, base: &Type) -> Diagnostic {
        if self.names_struct(base) {
            error.with_help(TypeProblems::struct_base_help())
        } else {
            error
        }
    }

    fn names_struct(&self, ty: &Type) -> bool {
        let declared = self.declared_types();
        matches!(self.unifier.shallow(ty), Type::Named(binding, _) if declared.is_struct(binding))
    }

    fn infer_update(
        &mut self,
        base: &Expr,
        fields: &[(FieldLabel, Expr)],
        span: Span,
    ) -> Result<(TypedExprKind, Type), Diagnostic> {
        let base = self.infer(base)?;
        let base = self.moved_field_base(base);
        let (binding, arguments) = self.update_owner(&base, fields, span)?;
        let typed_fields = self.check_fields(binding, &arguments, fields)?;
        Ok((
            TypedExprKind::Struct(TypedStructExpr::Update(
                binding,
                Box::new(base),
                typed_fields,
            )),
            Type::Named(binding, arguments),
        ))
    }

    fn moved_field_base(&mut self, base: TypedExpr) -> TypedExpr {
        if self.reads_owned_field(&base) {
            TypedExpr {
                ty: self.place_types().moved(&base, &self.copy_rules()),
                ..base
            }
        } else {
            base
        }
    }

    fn reads_owned_field(&self, base: &TypedExpr) -> bool {
        base.field_place().is_some_and(|place| {
            place.reads_field()
                && !matches!(
                    self.unifier.shallow(place.root_type),
                    Type::Ref(..) | Type::Mut(..)
                )
        })
    }

    fn update_owner(
        &mut self,
        base: &TypedExpr,
        fields: &[(FieldLabel, Expr)],
        span: Span,
    ) -> Result<(BindingId, Vec<Type>), Diagnostic> {
        let owner = self.unifier.shallow(&base.ty);
        match owner {
            Type::Named(binding, arguments) if self.declared_types().is_struct(binding) => {
                Ok((binding, arguments))
            }
            Type::Var(_) => self.update_owner_from_fields(&owner, fields, span),
            Type::Ref(_, inner) | Type::Mut(_, inner) if self.names_struct(&inner) => Err(
                TypeProblems::borrowed_update_base(&self.display().show(&inner), base.span),
            ),
            other => Err(self.field_bound().unbound_update(
                &self.display().show(&other),
                &other,
                base.span,
            )),
        }
    }

    fn update_owner_from_fields(
        &mut self,
        owner: &Type,
        fields: &[(FieldLabel, Expr)],
        span: Span,
    ) -> Result<(BindingId, Vec<Type>), Diagnostic> {
        let first = fields
            .first()
            .expect("the expander requires a field in an update");
        let candidates: Vec<BindingId> = self
            .declarations
            .types()
            .structs
            .with_field(&first.0.name)
            .into_iter()
            .filter(|binding| {
                let definition = self
                    .declarations
                    .types()
                    .structs
                    .get(*binding)
                    .expect("a struct with a field has a definition");
                fields
                    .iter()
                    .all(|(label, _)| definition.field(&label.name).is_some())
            })
            .collect();
        let [binding] = candidates.as_slice() else {
            return Err(FieldOwnerProblems::ambiguous(&first.0.name, first.0.span));
        };
        let arguments = self.fresh_arguments(*binding);
        let expected = Type::Named(*binding, arguments.clone());
        self.unification()
            .expect(Expectation::plain(&expected), owner, span)?;
        Ok((*binding, arguments))
    }

    fn check_fields(
        &mut self,
        binding: BindingId,
        arguments: &[Type],
        fields: &[(FieldLabel, Expr)],
    ) -> Result<Vec<(FieldLabel, TypedExpr)>, Diagnostic> {
        let struct_name = self.bindings.get(binding).name().to_string();
        let declared = self.instantiated_fields(binding, arguments);
        let mut typed_fields: Vec<(FieldLabel, TypedExpr)> = Vec::new();
        for (given, (label, value)) in fields.iter().enumerate() {
            let position = Self::field_position(&struct_name, &declared, fields, given)?;
            let field_type = &declared[position].1;
            let typed = self.infer_expecting(value, field_type, &MismatchContext::Plain)?;
            self.unification()
                .expect(Expectation::plain(field_type), &typed.ty, typed.span)?;
            typed_fields.push((label.clone(), typed));
        }
        Ok(typed_fields)
    }

    fn field_position(
        owner: &str,
        declared: &[(Rc<str>, Type)],
        fields: &[(FieldLabel, Expr)],
        given: usize,
    ) -> Result<usize, Diagnostic> {
        let label = &fields[given].0;
        if let Some((first, _)) = fields[..given]
            .iter()
            .find(|(earlier, _)| earlier.name == label.name)
        {
            return Err(TypeProblems::repeated_field(owner, label, first.span));
        }
        declared
            .iter()
            .position(|(name, _)| *name == label.name)
            .ok_or_else(|| {
                let names: Vec<Rc<str>> = declared.iter().map(|(name, _)| name.clone()).collect();
                TypeProblems::unknown_field(owner, label, SimilarName::closest(&label.name, &names))
            })
    }

    fn require_every_field(
        owner: &str,
        declared: &[(Rc<str>, Type)],
        fields: &[(FieldLabel, Expr)],
        span: Span,
    ) -> Result<(), Diagnostic> {
        let missing: Vec<Rc<str>> = declared
            .iter()
            .map(|(name, _)| name.clone())
            .filter(|name| !fields.iter().any(|(label, _)| label.name == *name))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(TypeProblems::missing_fields(owner, &missing, span))
    }

    fn instantiated_fields(&self, binding: BindingId, arguments: &[Type]) -> Vec<(Rc<str>, Type)> {
        self.declarations
            .types()
            .structs
            .get(binding)
            .map(|definition| definition.instantiated_fields(arguments))
            .unwrap_or_default()
    }

    fn fresh_arguments(&mut self, binding: BindingId) -> Vec<Type> {
        let count = self.type_arities.get(&binding).copied().unwrap_or_default();
        (0..count).map(|_| self.unifier.fresh()).collect()
    }

    fn infer_field(
        &mut self,
        value: &Expr,
        label: &FieldLabel,
        span: Span,
    ) -> Result<TypedExpr, Diagnostic> {
        let value = self.infer(value)?;
        let read = self.field_bound().read(&value.ty, &label.name);
        if let Some(ty) = read.or_else(|| self.open_field_read().infer(&value.ty, label)) {
            return Ok(FieldReadExpr::new(value, label).typed(ty, span));
        }
        let (owner, arguments) = self.field_owner(&value, label)?;
        let reads = self.open_read();
        let declared = (self.declarations.types().structs.get(owner))
            .and_then(|definition| reads.instantiated(definition, &label.name, &arguments));
        let Some(read) = declared else {
            let struct_name = self.bindings.get(owner).name().to_string();
            let structs = &self.declarations.types().structs;
            let similar = structs
                .get(owner)
                .and_then(|found| found.closest_field(&label.name));
            return Err(TypeProblems::unknown_field(&struct_name, label, similar));
        };
        self.record_macro_field_read(label, span);
        Ok(FieldReadExpr::new(value, label).typed(read.read, span))
    }

    fn field_owner(
        &mut self,
        value: &TypedExpr,
        label: &FieldLabel,
    ) -> Result<(BindingId, Vec<Type>), Diagnostic> {
        let mut owner = self.unifier.shallow(&value.ty);
        while let Type::Ref(_, inner) | Type::Mut(_, inner) = owner {
            owner = self.unifier.shallow(&inner);
        }
        match owner {
            Type::Named(binding, arguments) if self.declared_types().is_struct(binding) => {
                Ok((binding, arguments))
            }
            Type::Var(var) => match self.unifier.literal_kind(var) {
                Some(literal) => Err(FieldOwnerProblems::literal_owner(literal, value.span)),
                None => self.unique_owner_or_request_trait(&owner, label),
            },
            other => {
                Err(self
                    .field_bound()
                    .unbound_read(&self.display().show(&other), value, label))
            }
        }
    }

    fn unique_owner_or_request_trait(
        &mut self,
        owner: &Type,
        label: &FieldLabel,
    ) -> Result<(BindingId, Vec<Type>), Diagnostic> {
        let candidates = self.open_field_read().candidates(label);
        match candidates.as_slice() {
            [binding] => {
                let arguments = self.fresh_arguments(*binding);
                let expected = Type::Named(*binding, arguments.clone());
                self.unification()
                    .expect(Expectation::plain(&expected), owner, label.span)?;
                Ok((*binding, arguments))
            }
            [] => Err(FieldOwnerProblems::undeclared(
                label,
                self.declarations.types().structs.closest_field(&label.name),
            )),
            _ => {
                self.open_field_read().request(&label.name);
                Err(FieldOwnerProblems::ambiguous(&label.name, label.span))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use rustc_hash::FxHashSet;

    use super::TypeChecker;
    use crate::core::{BindingArena, BindingInfo, BindingKind, ModuleId, PrimitiveType};
    use crate::source::{SourceId, Span};
    use crate::types::definition_outcomes::DefinitionOutcomes;
    use crate::types::forms::{ExpressionForm, VariableReference};
    use crate::types::{ClosureKind, Type, TypeVar};

    #[test]
    fn a_function_error_with_an_open_part_does_not_convert() {
        let payload = Type::Function(
            ClosureKind::Fn,
            vec![Type::Primitive(PrimitiveType::I64)],
            Box::new(Type::Primitive(PrimitiveType::I64)),
        );
        let error = Type::Function(
            ClosureKind::Fn,
            vec![Type::Primitive(PrimitiveType::I64)],
            Box::new(Type::Var(TypeVar::new(0))),
        );
        assert!(!TypeChecker::could_unify(&payload, &error));
    }

    struct PlaceTypeFixture {
        checker: TypeChecker,
        span: Span,
    }

    impl PlaceTypeFixture {
        fn new() -> Self {
            Self {
                checker: TypeChecker::new::<crate::types::DroppedInlineSchemes>(
                    BindingArena::default(),
                ),
                span: Span::new(SourceId::new(0), 0..0),
            }
        }

        fn global_without_a_scheme(&mut self) -> Type {
            let binding = self.checker.bindings.add(BindingInfo::new(
                "missing".into(),
                BindingKind::Global(ModuleId::new(0)),
                None,
            ));
            let span = Span::new(SourceId::new(0), 0..0);
            self.checker.global_type(binding, span)
        }

        fn type_of_an_unreached_prelude_definition(&mut self) -> Type {
            let binding = self.checker.bindings.add(BindingInfo::new(
                "missed".into(),
                BindingKind::Global(ModuleId::new(0)),
                None,
            ));
            self.checker.outcomes =
                DefinitionOutcomes::new(Rc::new(FxHashSet::from_iter([binding])));
            let span = Span::new(SourceId::new(0), 0..0);
            self.checker.global_type(binding, span)
        }

        fn rejects_a_compile_time_only_global(&mut self) -> bool {
            let binding = self.checker.bindings.add(BindingInfo::new(
                "hidden".into(),
                BindingKind::Global(ModuleId::new(0)),
                None,
            ));
            self.checker.unchecked.insert(binding);
            VariableReference::new(binding, self.span)
                .type_against(&mut self.checker, None)
                .is_err()
        }
    }

    #[test]
    fn a_global_without_a_scheme_gets_a_fresh_type() {
        let mut fixture = PlaceTypeFixture::new();
        assert!(matches!(fixture.global_without_a_scheme(), Type::Var(_)));
    }

    #[test]
    #[should_panic(expected = "the definition graph missed a reference")]
    fn a_reference_to_an_unreached_prelude_definition_panics() {
        PlaceTypeFixture::new().type_of_an_unreached_prelude_definition();
    }

    #[test]
    fn a_compile_time_only_global_is_rejected() {
        let mut fixture = PlaceTypeFixture::new();
        assert!(fixture.rejects_a_compile_time_only_global());
    }
}
