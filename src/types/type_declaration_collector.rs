use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::collected_enum::CollectedEnum;
use super::collected_struct::CollectedStruct;
use super::declared_types::DeclaredTypes;
use super::enum_item::EnumItem;
use super::enums::{EnumDefinition, VariantDefinition, VariantPayload};
use super::field_parameters::FieldParameters;
use super::lowered_payload::LoweredPayload;
use super::parameter_usage::ParameterUsage;
use super::payload_lowering::PayloadLowering;
use super::problems::TypeProblems;
use super::rust_type_item::RustTypeItem;
use super::ty::{GenericId, GenericParam, Type};
use super::type_declarations::TypeDeclarations;
use super::written_type_lowering::WrittenTypeLowering;
use super::{RustType, StructDefinition};
use crate::core::{
    BindingArena, BindingId, FieldLabel, TypeBinder, TypeExpr, TypeExprKind,
    VariantPayload as CoreVariantPayload,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

type LoweredFields = Vec<(Rc<str>, Type)>;

#[derive(Default)]
struct LoweredStructFields {
    fields: LoweredFields,
    spans: Vec<Span>,
    bare_text: Vec<Rc<str>>,
}

pub(super) struct TypeDeclarationCollector<'c> {
    lowering: WrittenTypeLowering<'c>,
    bindings: &'c BindingArena,
    types: &'c TypeDeclarations,
}

impl<'c> TypeDeclarationCollector<'c> {
    pub(super) fn new(
        lowering: WrittenTypeLowering<'c>,
        bindings: &'c BindingArena,
        types: &'c TypeDeclarations,
    ) -> Self {
        Self {
            lowering,
            bindings,
            types,
        }
    }

    pub(super) fn collect_struct(
        &mut self,
        binders: &[TypeBinder],
        fields: &[(FieldLabel, TypeExpr)],
    ) -> Result<CollectedStruct, Diagnostic> {
        let params = self.struct_params(binders)?;
        let lifetimes = Self::lifetime_binders(binders);
        let scope: FxHashMap<BindingId, GenericId> = binders
            .iter()
            .filter(|binder| !binder.lifetime)
            .zip(&params)
            .map(|(binder, param)| (binder.variable, param.id))
            .collect();
        let LoweredStructFields {
            fields: lowered,
            spans: field_spans,
            bare_text: bare_text_fields,
        } = self.lower_fields(fields, &scope)?;
        let problem = FieldParameters::new(binders, &params).unused_in(&lowered);
        let definition = StructDefinition {
            lifetimes,
            params,
            fields: lowered,
            field_spans,
            bare_text_fields,
            function_spans: fields
                .iter()
                .map(|(_, field_type)| Self::function_span(field_type))
                .collect(),
            derives: Vec::new(),
            derived_traits: Vec::new(),
            blocked: Vec::new(),
            written_derive: None,
        };
        Ok(CollectedStruct::new(definition, problem))
    }

    pub(super) fn collect_enum(&mut self, item: EnumItem<'_>) -> Result<CollectedEnum, Diagnostic> {
        let EnumItem {
            binders,
            variants,
            rust,
            definer,
            ..
        } = item;
        let params = self.struct_params(binders)?;
        let lifetimes = Self::lifetime_binders(binders);
        let scope: FxHashMap<BindingId, GenericId> = binders
            .iter()
            .filter(|binder| !binder.lifetime)
            .zip(&params)
            .map(|(binder, param)| (binder.variable, param.id))
            .collect();
        let mut lowered = Vec::new();
        let mut problems = Vec::new();
        for variant in variants {
            let payload = self.lower_variant_payload(&variant.payload, &scope);
            let lowering = PayloadLowering::of(&payload.problems);
            problems.extend(payload.problems);
            let type_exprs = Self::payload_type_exprs(&variant.payload);
            lowered.push(VariantDefinition {
                name: variant.name.clone(),
                payload: payload.payload,
                lowering,
                payload_spans: type_exprs
                    .iter()
                    .map(|type_expr| type_expr.span())
                    .collect(),
                function_spans: type_exprs.into_iter().map(Self::function_span).collect(),
                span: variant.span,
            });
        }
        problems.extend(self.check_enum_parameters_used(item, &params));
        let rust_type = rust.as_ref().map(|rust| RustType {
            template: rust.template.clone(),
            clauses: Vec::new(),
            params: params.iter().map(|param| Rc::clone(&param.name)).collect(),
            unsized_parameters: Vec::new(),
        });
        let definition = EnumDefinition {
            lifetimes,
            params,
            variants: lowered,
            rust: rust.clone(),
            definer: definer.clone(),
            derives: Vec::new(),
            derived_traits: Vec::new(),
            blocked: Vec::new(),
            written_derive: None,
        };
        Ok(CollectedEnum::new(definition, rust_type, problems))
    }

    pub(super) fn collect_rust_type(
        &mut self,
        item: RustTypeItem<'_>,
    ) -> Result<RustType, Diagnostic> {
        let RustTypeItem {
            binders,
            unsized_parameters,
            template,
            clauses,
        } = item;
        self.reject_bounds(binders)?;
        let clauses = clauses
            .iter()
            .map(|(target, template)| {
                Ok((
                    self.lowering.lower_without_generics(target)?,
                    template.clone(),
                ))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        Ok(RustType {
            template: template.clone(),
            clauses,
            params: binders
                .iter()
                .filter(|binder| !binder.lifetime)
                .map(|binder| Rc::from(self.bindings.get(binder.variable).name()))
                .collect(),
            unsized_parameters: unsized_parameters.to_vec(),
        })
    }

    fn lower_variant_payload(
        &mut self,
        payload: &CoreVariantPayload,
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> LoweredPayload {
        let mut problems = Vec::new();
        let mut lower = |type_expr: &TypeExpr| {
            self.lower_stored_type(type_expr, scope)
                .unwrap_or_else(|problem| {
                    problems.push(problem);
                    self.lowering.placeholder_type()
                })
        };
        let payload = match payload {
            CoreVariantPayload::None => VariantPayload::None,
            CoreVariantPayload::Positional(types) => {
                VariantPayload::Positional(types.iter().map(&mut lower).collect())
            }
            CoreVariantPayload::Named(fields) => VariantPayload::Named(
                fields
                    .iter()
                    .map(|(label, field_type)| (label.name.clone(), lower(field_type)))
                    .collect(),
            ),
        };
        LoweredPayload { payload, problems }
    }

    fn check_enum_parameters_used(
        &self,
        item: EnumItem<'_>,
        params: &[GenericParam],
    ) -> Option<Diagnostic> {
        let enum_name = self.bindings.get(item.binding).name();
        item.binders
            .iter()
            .filter(|binder| !binder.lifetime)
            .zip(params)
            .find_map(|(binder, param)| {
                let usage = ParameterUsage::scan(item.variants, item.binding, binder.variable);
                match (usage.used_outside_self(), usage.first_recursive_use()) {
                    (true, _) => None,
                    (false, Some(recursive_use)) => {
                        Some(TypeProblems::recursively_used_enum_parameter(
                            &param.name,
                            enum_name,
                            binder.span,
                            recursive_use,
                        ))
                    }
                    (false, None) => Some(TypeProblems::unused_enum_parameter(
                        &param.name,
                        binder.span,
                    )),
                }
            })
    }

    fn lower_fields(
        &mut self,
        fields: &[(FieldLabel, TypeExpr)],
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<LoweredStructFields, Diagnostic> {
        let mut lowered = LoweredStructFields::default();
        for (label, field_type) in fields {
            let written = self.lowering.lower_type(field_type, scope)?;
            let stored = self.borrow_views(written.clone());
            if stored != written {
                lowered.bare_text.push(label.name.clone());
            }
            lowered.fields.push((label.name.clone(), stored));
            lowered.spans.push(field_type.span());
        }
        Ok(lowered)
    }

    fn payload_type_exprs(payload: &CoreVariantPayload) -> Vec<&TypeExpr> {
        match payload {
            CoreVariantPayload::None => Vec::new(),
            CoreVariantPayload::Positional(types) => types.iter().collect(),
            CoreVariantPayload::Named(fields) => {
                fields.iter().map(|(_, field_type)| field_type).collect()
            }
        }
    }

    fn function_span(stored_type: &TypeExpr) -> Span {
        Self::inner_function_span(stored_type).unwrap_or(stored_type.span())
    }

    fn inner_function_span(type_expr: &TypeExpr) -> Option<Span> {
        if matches!(type_expr.kind(), TypeExprKind::Function(..)) {
            return Some(type_expr.span());
        }
        type_expr
            .parts()
            .into_iter()
            .find_map(Self::inner_function_span)
    }

    fn lower_stored_type(
        &mut self,
        stored_type: &TypeExpr,
        scope: &FxHashMap<BindingId, GenericId>,
    ) -> Result<Type, Diagnostic> {
        let lowered = self.lowering.lower_type(stored_type, scope)?;
        Ok(self.borrow_views(lowered))
    }

    fn borrow_views(&self, stored: Type) -> Type {
        match stored {
            view if DeclaredTypes::new(self.types).is_unsized(&view) => Type::shared(view),
            Type::Named(binding, arguments) => {
                Type::Named(binding, self.borrow_named_arguments(binding, arguments))
            }
            sized => sized,
        }
    }

    fn borrow_argument_views(&self, argument: Type) -> Type {
        match argument {
            Type::Named(binding, arguments) => {
                Type::Named(binding, self.borrow_named_arguments(binding, arguments))
            }
            view => view,
        }
    }

    fn borrow_named_arguments(&self, binding: BindingId, arguments: Vec<Type>) -> Vec<Type> {
        arguments
            .into_iter()
            .enumerate()
            .map(|(position, argument)| {
                if self
                    .types
                    .rust_types
                    .parameter_may_be_unsized(binding, position)
                {
                    self.borrow_argument_views(argument)
                } else {
                    self.borrow_views(argument)
                }
            })
            .collect()
    }

    fn struct_params(&mut self, binders: &[TypeBinder]) -> Result<Vec<GenericParam>, Diagnostic> {
        self.reject_bounds(binders)?;
        Ok(binders
            .iter()
            .filter(|binder| !binder.lifetime)
            .map(|binder| self.lowering.declare_variable(binder.variable))
            .collect())
    }

    fn lifetime_binders(binders: &[TypeBinder]) -> Vec<BindingId> {
        binders
            .iter()
            .filter(|binder| binder.lifetime)
            .map(|binder| binder.variable)
            .collect()
    }

    fn reject_bounds(&self, binders: &[TypeBinder]) -> Result<(), Diagnostic> {
        match binders.iter().find(|binder| !binder.bounds.is_empty()) {
            Some(binder) => Err(TypeProblems::bounded_type_parameter(
                self.bindings.get(binder.variable).name(),
                binder.span,
            )),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::core::{EnumRust, ParsedTemplate, VariantPayload};
    use crate::types::enum_item::EnumItem;
    use crate::types::rust_type_item::RustTypeItem;
    use crate::types::ty::Type;
    use crate::types::written_type_fixture::WrittenTypeFixture as Fixture;

    #[test]
    fn a_struct_keeps_its_lifetimes_and_lowers_fields_with_its_params() {
        let mut fixture = Fixture::new();
        let pair = fixture.declared_struct("pair", 1);
        let lifetime = fixture.lifetime("'a");
        let variable = fixture.type_variable("t");
        let binders = [
            Fixture::lifetime_binder(lifetime),
            Fixture::bounded(variable, &[]),
        ];
        let fields = [(
            Fixture::field_label("value"),
            Fixture::named(variable, Vec::new()),
        )];
        let collected = fixture
            .checker
            .type_collector()
            .collect_struct(&binders, &fields)
            .expect("the struct collects");
        fixture.enter_struct(pair, collected);
        let definition = fixture.checker.declarations.types().structs.get(pair);
        let definition = definition.expect("the struct is entered");
        assert_eq!(definition.lifetimes, vec![lifetime]);
        assert_eq!(definition.params.len(), 1);
        assert_eq!(
            definition.fields[0].1,
            Type::Generic(definition.params[0].id)
        );
        assert!(fixture.checker.diagnostics.is_empty());
    }

    #[test]
    fn a_struct_parameter_no_field_uses_is_reported() {
        let mut fixture = Fixture::new();
        let counter = fixture.declared_struct("counter", 1);
        let variable = fixture.type_variable("t");
        let binders = [Fixture::bounded(variable, &[])];
        let fields = [(Fixture::field_label("count"), Fixture::integer())];
        let collected = fixture
            .checker
            .type_collector()
            .collect_struct(&binders, &fields)
            .expect("the struct collects");
        fixture.enter_struct(counter, collected);
        assert_eq!(fixture.checker.diagnostics.len(), 1);
    }

    #[test]
    fn a_bounded_struct_parameter_is_refused() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("t");
        let display = fixture.named_trait("display");
        let binders = [Fixture::bounded(variable, &[display])];
        let collected = fixture
            .checker
            .type_collector()
            .collect_struct(&binders, &[]);
        assert!(collected.is_err());
    }

    #[test]
    fn an_enum_keeps_its_lifetimes() {
        let mut fixture = Fixture::new();
        let lifetime = fixture.lifetime("'a");
        let shape = fixture.declared_struct("shape", 0);
        let variants = [fixture.variant("empty", VariantPayload::None)];
        let binders = [Fixture::lifetime_binder(lifetime)];
        let collected = fixture
            .checker
            .type_collector()
            .collect_enum(EnumItem {
                binding: shape,
                binders: &binders,
                variants: &variants,
                rust: &None,
                definer: &None,
            })
            .expect("the enum collects");
        fixture.enter_enum(shape, collected);
        let types = fixture.checker.declarations.types();
        let definition = types.enums.get(shape).expect("the enum is entered");
        assert_eq!(definition.lifetimes, vec![lifetime]);
        assert!(types.rust_types.get(shape).is_none());
        assert!(fixture.checker.diagnostics.is_empty());
    }

    #[test]
    fn an_enum_with_a_rust_form_enters_its_rust_type() {
        let mut fixture = Fixture::new();
        let shape = fixture.declared_struct("shape", 0);
        let variants = [fixture.variant("empty", VariantPayload::None)];
        let rust = Some(EnumRust {
            path: "Shape".into(),
            template: Rc::new(ParsedTemplate::parse("Shape").expect("a template")),
            template_clause: false,
            bare: false,
        });
        let collected = fixture
            .checker
            .type_collector()
            .collect_enum(EnumItem {
                binding: shape,
                binders: &[],
                variants: &variants,
                rust: &rust,
                definer: &None,
            })
            .expect("the enum collects");
        fixture.enter_enum(shape, collected);
        assert!(
            fixture
                .checker
                .declarations
                .types()
                .rust_types
                .get(shape)
                .is_some()
        );
    }

    #[test]
    fn a_payload_that_fails_to_lower_becomes_a_placeholder_generic() {
        let mut fixture = Fixture::new();
        let shape = fixture.declared_struct("shape", 0);
        let point = fixture.declared_struct("point", 0);
        let wrong = Fixture::named(point, vec![Fixture::integer()]);
        let variants = [fixture.variant("bad", VariantPayload::Positional(vec![wrong]))];
        let collected = fixture
            .checker
            .type_collector()
            .collect_enum(EnumItem {
                binding: shape,
                binders: &[],
                variants: &variants,
                rust: &None,
                definer: &None,
            })
            .expect("the enum collects");
        fixture.enter_enum(shape, collected);
        assert_eq!(fixture.checker.diagnostics.len(), 1);
        let definition = fixture.checker.declarations.types().enums.get(shape);
        let definition = definition.expect("the enum is entered");
        let crate::types::enums::VariantPayload::Positional(payload) =
            &definition.variants[0].payload
        else {
            panic!("a positional payload stays positional");
        };
        assert!(matches!(payload[0], Type::Generic(_)));
    }
    #[test]
    fn a_rust_type_keeps_its_clauses_and_unsized_parameters() {
        let mut fixture = Fixture::new();
        let variable = fixture.type_variable("t");
        let template = std::rc::Rc::new(ParsedTemplate::parse("Holder").expect("a template"));
        let clause = std::rc::Rc::new(ParsedTemplate::parse("Holder<i64>").expect("a template"));
        let binders = [Fixture::bounded(variable, &[])];
        let clauses = [(Fixture::integer(), clause)];
        let rust_type = fixture
            .checker
            .type_collector()
            .collect_rust_type(RustTypeItem {
                binders: &binders,
                unsized_parameters: &[0],
                template: &template,
                clauses: &clauses,
            })
            .expect("the rust type collects");
        assert_eq!(rust_type.clauses.len(), 1);
        assert_eq!(rust_type.unsized_parameters, vec![0]);
        assert_eq!(rust_type.params.len(), 1);
    }
}
