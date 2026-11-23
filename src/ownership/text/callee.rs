use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::closure_slots::ClosureUse;
use super::function_value::FunctionValue;
use super::inline_declaration::InlineDeclaration;
use super::inline_rust::InlineRust;
use super::map_queries::MapQueries;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::template_call::TemplateCall;
use super::text_program::TextProgram;
use super::user_function::UserFunction;
use crate::core::{BindingId, RustTemplate};
use crate::types::{Type, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf};

pub struct Callee;

impl Callee {
    pub fn of<'t>(
        callee: &'t TypedExpr,
        program: &TextProgram<'t>,
    ) -> Option<Box<dyn CallTarget<'t> + 't>> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                Self::global(*binding, (callee, program))
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => {
                Some(Self::inline(template, (callee, program)))
            }
            _ => None,
        }
    }

    fn global<'t>(
        binding: BindingId,
        (callee, program): (&'t TypedExpr, &TextProgram<'t>),
    ) -> Option<Box<dyn CallTarget<'t> + 't>> {
        match (
            program.definition(binding),
            program.written().trait_method(binding),
        ) {
            (_, Some(declared)) => Some(Box::new(FunctionValue::new(
                CalleeScheme::new(&callee.ty, &[]),
                Slot::from(SlotOwner::TraitMethod(declared)),
            ))),
            (Some(definition), None) => Some(Self::defined(definition)),
            (None, None) => None,
        }
    }

    fn inline<'t>(
        template: &RustTemplate,
        (callee, program): (&'t TypedExpr, &TextProgram<'t>),
    ) -> Box<dyn CallTarget<'t> + 't> {
        match InlineDeclaration::of(&template.type_expr) {
            Some(declared) => Box::new(TemplateCall::new(CalleeScheme::written(declared))),
            None => Box::new(InlineRust::new(
                CalleeScheme::new(&callee.ty, &[]),
                Self::map_queries(callee, program),
            )),
        }
    }

    pub fn fixes_closures(callee: &TypedExpr, program: &TextProgram<'_>) -> ClosureUse {
        let template = match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Rust(_)) => true,
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => program
                .definition(*binding)
                .is_some_and(|definition| !matches!(definition.value.kind, TypedExprKind::Fn(_))),
            _ => false,
        };
        if template {
            ClosureUse::Fixed
        } else {
            ClosureUse::Free
        }
    }

    fn map_queries(callee: &TypedExpr, program: &TextProgram<'_>) -> Vec<usize> {
        match &callee.ty {
            Type::Function(_, parameters, _) => MapQueries::new(program.types()).of(parameters),
            _ => Vec::new(),
        }
    }

    fn defined<'t>(definition: &'t TypedDefinition) -> Box<dyn CallTarget<'t> + 't> {
        let scheme = CalleeScheme::new(&definition.scheme.body, &definition.scheme.params);
        match &definition.value.kind {
            TypedExprKind::Fn(lambda) => {
                Box::new(UserFunction::new(scheme, definition.binding, lambda))
            }
            _ => Box::new(TemplateCall::new(scheme)),
        }
    }
}
