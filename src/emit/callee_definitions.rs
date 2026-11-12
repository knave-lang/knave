use std::rc::Rc;

use super::plan::EmitPlan;
use crate::core::{BindingId, RustTemplate};
use crate::types::{TypedExpr, TypedExprKind, TypedLeaf};

pub enum CalleeDefinition<'c> {
    Function,
    Template(&'c Rc<RustTemplate>),
    Unknown,
}

#[derive(Clone, Copy)]
pub struct CalleeDefinitions<'p> {
    plan: &'p EmitPlan<'p>,
}

impl<'p> CalleeDefinitions<'p> {
    pub fn new(plan: &'p EmitPlan<'p>) -> Self {
        Self { plan }
    }

    pub fn of<'c>(&self, callee: &'c TypedExpr) -> CalleeDefinition<'c>
    where
        'p: 'c,
    {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => CalleeDefinition::Template(template),
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => self.of_definition(*binding),
            _ => CalleeDefinition::Unknown,
        }
    }

    pub fn of_definition(&self, binding: BindingId) -> CalleeDefinition<'p> {
        match self
            .plan
            .definition(binding)
            .map(|definition| &definition.value.kind)
        {
            Some(TypedExprKind::Fn(_)) => CalleeDefinition::Function,
            Some(TypedExprKind::Leaf(TypedLeaf::Rust(template))) => {
                CalleeDefinition::Template(template)
            }
            _ => CalleeDefinition::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CalleeDefinition, CalleeDefinitions};
    use crate::emit::test_program::TestProgram;

    struct CalleeFixture {
        program: TestProgram,
    }

    impl CalleeFixture {
        const SOURCE: &str = "
            (type first-letter (fn ((ref str)) (ref str)))
            (defn first-letter (text) (subs text 0 1))
            (def trimmed (rust \"{0}.trim()\" (fn ((ref str)) (ref str))))
            (defn function () (local t (first-letter \"ab\")) t)
            (defn defined-template () (local t (trimmed \"ab\")) t)
            (defn written-template ()
              (local t ((rust \"{0}.trim()\" (fn ((ref str)) (ref str))) \"ab\"))
              t)
            (defn local-callee () (local f first-letter) (local t (f \"ab\")) t)
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TestProgram::new(Self::SOURCE),
            }
        }

        fn kind(&self, function: &str) -> &'static str {
            let plan = self.program.plan();
            let callee = TestProgram::callee(self.program.last_let_value(function));
            match CalleeDefinitions::new(&plan).of(callee) {
                CalleeDefinition::Function => "function",
                CalleeDefinition::Template(_) => "template",
                CalleeDefinition::Unknown => "unknown",
            }
        }
    }

    #[test]
    fn a_defn_name_is_a_function() {
        assert_eq!(CalleeFixture::new().kind("function"), "function");
    }

    #[test]
    fn a_defined_rust_template_is_a_template() {
        assert_eq!(CalleeFixture::new().kind("defined-template"), "template");
    }

    #[test]
    fn a_written_rust_template_is_a_template() {
        assert_eq!(CalleeFixture::new().kind("written-template"), "template");
    }

    #[test]
    fn a_local_callee_is_unknown() {
        assert_eq!(CalleeFixture::new().kind("local-callee"), "unknown");
    }
}
