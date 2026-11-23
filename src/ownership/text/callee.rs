use std::rc::Rc;

use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::closure_slots::ClosureUse;
use super::function_value::FunctionValue;
use super::inline_rust::InlineRust;
use super::map_queries::MapQueries;
use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::template_call::TemplateCall;
use super::text_program::TextProgram;
use super::user_function::UserFunction;
use crate::core::BindingId;
use crate::types::{Scheme, Type, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf};

pub struct Callee<'c, 't> {
    program: &'c TextProgram<'t>,
}

impl<'c, 't> Callee<'c, 't> {
    pub fn new(program: &'c TextProgram<'t>) -> Self {
        Self { program }
    }

    pub fn of(&self, callee: &'t TypedExpr) -> Option<Box<dyn CallTarget<'t> + 't>> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => self.global(*binding, callee),
            TypedExprKind::Leaf(TypedLeaf::Rust(_, scheme)) => {
                Some(self.inline(scheme.as_ref(), callee))
            }
            _ => None,
        }
    }

    pub fn closure_use(&self, callee: &TypedExpr) -> ClosureUse {
        let template = match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Rust(..)) => true,
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => self
                .program
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

    fn global(
        &self,
        binding: BindingId,
        callee: &'t TypedExpr,
    ) -> Option<Box<dyn CallTarget<'t> + 't>> {
        match (
            self.program.definition(binding),
            self.program.written().trait_method(binding),
        ) {
            (_, Some(declared)) => Some(Box::new(FunctionValue::new(
                CalleeScheme::new(&callee.ty, &[]),
                Slot::from(SlotOwner::TraitMethod(declared)),
            ))),
            (Some(definition), None) => Some(Self::defined(definition)),
            (None, None) => None,
        }
    }

    fn inline(
        &self,
        scheme: Option<&'t Rc<Scheme>>,
        callee: &'t TypedExpr,
    ) -> Box<dyn CallTarget<'t> + 't> {
        match scheme {
            Some(scheme) => Box::new(TemplateCall::new(CalleeScheme::new(
                &scheme.body,
                &scheme.params,
            ))),
            None => Box::new(InlineRust::new(
                CalleeScheme::new(&callee.ty, &[]),
                self.map_queries(callee),
            )),
        }
    }

    fn map_queries(&self, callee: &TypedExpr) -> Vec<usize> {
        match &callee.ty {
            Type::Function(_, parameters, _) => {
                MapQueries::new(self.program.types()).of(parameters)
            }
            _ => Vec::new(),
        }
    }

    fn defined(definition: &'t TypedDefinition) -> Box<dyn CallTarget<'t> + 't> {
        let scheme = CalleeScheme::new(&definition.scheme.body, &definition.scheme.params);
        match &definition.value.kind {
            TypedExprKind::Fn(lambda) => {
                Box::new(UserFunction::new(scheme, definition.binding, lambda))
            }
            _ => Box::new(TemplateCall::new(scheme)),
        }
    }
}
