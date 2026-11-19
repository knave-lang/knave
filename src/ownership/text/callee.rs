use super::call_target::CallTarget;
use super::callee_scheme::CalleeScheme;
use super::inline_rust::InlineRust;
use super::template_call::TemplateCall;
use super::text_program::TextProgram;
use super::user_function::UserFunction;
use crate::types::{SyntheticTemplate, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf};

pub struct Callee;

impl Callee {
    pub fn of<'t>(
        callee: &'t TypedExpr,
        program: &TextProgram<'t>,
    ) -> Option<Box<dyn CallTarget<'t> + 't>> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                program.definition(*binding).map(Self::defined)
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => Some(Box::new(InlineRust::new(
                CalleeScheme::new(&callee.ty, &[]),
                SyntheticTemplate::named(&template.template)
                    .map_or(&[], SyntheticTemplate::queries),
            ))),
            _ => None,
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
