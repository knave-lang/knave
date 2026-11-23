use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::{BindingId, ModuleId, RustTemplate};
use crate::types::{
    ClauseChoice, TypedDefinition, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
};

pub struct ConstantDefinitions<'p, 'd> {
    program: &'p TypedProgram,
    definitions: &'d FxHashMap<BindingId, (&'p TypedDefinition, ModuleId)>,
}

impl<'p, 'd> ConstantDefinitions<'p, 'd> {
    const MACRO_CALLS: [&'static str; 3] = ["!(", "![", "!{"];

    pub fn new(
        program: &'p TypedProgram,
        definitions: &'d FxHashMap<BindingId, (&'p TypedDefinition, ModuleId)>,
    ) -> Self {
        Self {
            program,
            definitions,
        }
    }

    pub fn is_const_evaluable(&self, expr: &TypedExpr) -> bool {
        match &expr.kind {
            TypedExprKind::Leaf(leaf) => self.is_const_leaf(leaf),
            TypedExprKind::Call(callee, arguments) => {
                self.is_const_callee(callee)
                    && arguments
                        .iter()
                        .all(|argument| self.is_const_evaluable(&argument.value))
            }
            _ => false,
        }
    }

    fn is_const_callee(&self, callee: &TypedExpr) -> bool {
        match self.called_template(callee) {
            Some(template) => Self::is_operator_template(&template.template),
            None => self.is_const_evaluable(callee),
        }
    }

    fn called_template(&self, callee: &TypedExpr) -> Option<Rc<RustTemplate>> {
        let TypedExprKind::Leaf(TypedLeaf::Global(binding)) = callee.kind else {
            return None;
        };
        let definition = self.definition(binding)?;
        let TypedExprKind::Leaf(TypedLeaf::Rust(template, _)) = &definition.value.kind else {
            return None;
        };
        Some(ClauseChoice::new(self.program).for_call(template, definition, &callee.ty))
    }

    fn is_const_leaf(&self, leaf: &TypedLeaf) -> bool {
        match leaf {
            TypedLeaf::Literal(_) => true,
            TypedLeaf::Global(binding) => self
                .definition(*binding)
                .is_some_and(|definition| self.is_const_evaluable(&definition.value)),
            TypedLeaf::Rust(template, _) => Self::is_operator_template(&template.template),
            TypedLeaf::Local(_) => false,
        }
    }

    fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.definitions
            .get(&binding)
            .map(|(definition, _)| *definition)
    }

    fn is_operator_template(text: &str) -> bool {
        let is_plain_value = text.chars().all(char::is_alphanumeric);
        let calls_something =
            Self::MACRO_CALLS.iter().any(|call| text.contains(call)) || text.contains(['.', '(']);
        is_plain_value || !calls_something
    }
}
