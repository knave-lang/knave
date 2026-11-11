use super::definition_queries::DefinitionQueries;
use super::expansion_callbacks::ExpansionCallbacks;
use super::global_values::GlobalValues;
use super::module_notes::ModuleNotes;
use super::module_queries::ModuleQueries;
use super::name_comparison::NameComparison;
use super::source_queries::SourceQueries;
use super::warnings::Warnings;

pub trait Host {
    fn global_values(&mut self) -> Option<&mut dyn GlobalValues>;
    fn module_queries(&mut self) -> Option<&mut dyn ModuleQueries>;
    fn definition_queries(&mut self) -> Option<&mut dyn DefinitionQueries>;
    fn module_notes(&mut self) -> Option<&mut dyn ModuleNotes>;
    fn name_comparison(&mut self) -> Option<&mut dyn NameComparison>;
    fn expansion_callbacks(&mut self) -> Option<&mut dyn ExpansionCallbacks>;
    fn source_queries(&mut self) -> Option<&mut dyn SourceQueries>;
    fn warnings(&mut self) -> Option<&mut dyn Warnings>;
}

#[cfg(test)]
mod tests {
    use super::{
        DefinitionQueries, ExpansionCallbacks, GlobalValues, Host, ModuleNotes, ModuleQueries,
        NameComparison, SourceQueries, Warnings,
    };
    use crate::core::{
        BindingArena, BindingInfo, BindingKind, Builtin, Expr, ExprKind, Leaf, ModuleId,
    };
    use crate::diagnostic::{Diagnostic, DiagnosticCode};
    use crate::interpret::{BuiltinCall, Interpreter, StepBudget, Value, WorkMeter};
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct NoGroups;

    impl Host for NoGroups {
        fn global_values(&mut self) -> Option<&mut dyn GlobalValues> {
            None
        }

        fn module_queries(&mut self) -> Option<&mut dyn ModuleQueries> {
            None
        }

        fn definition_queries(&mut self) -> Option<&mut dyn DefinitionQueries> {
            None
        }

        fn module_notes(&mut self) -> Option<&mut dyn ModuleNotes> {
            None
        }

        fn name_comparison(&mut self) -> Option<&mut dyn NameComparison> {
            None
        }

        fn expansion_callbacks(&mut self) -> Option<&mut dyn ExpansionCallbacks> {
            None
        }

        fn source_queries(&mut self) -> Option<&mut dyn SourceQueries> {
            None
        }

        fn warnings(&mut self) -> Option<&mut dyn Warnings> {
            None
        }
    }

    struct RefusalFixture;

    impl RefusalFixture {
        fn span() -> Span {
            Span::new(SourceId::new(0), 3..9)
        }

        fn symbol(name: &str) -> Value {
            Value::Form(Form::new(FormKind::Symbol(Symbol::new(name)), Self::span()))
        }

        fn same_binding_without_names() -> Diagnostic {
            let meter = WorkMeter::new(StepBudget::full(), Self::span());
            let arguments = vec![Self::symbol("left"), Self::symbol("right")];
            BuiltinCall::new(Builtin::SameBinding, arguments, Self::span(), &meter)
                .run(&mut NoGroups)
                .expect_err("no host group compares names")
        }

        fn global_read_without_globals() -> Diagnostic {
            let binding = BindingArena::default().add(BindingInfo::new(
                "origin".into(),
                BindingKind::Global(ModuleId::new(0)),
                None,
            ));
            let read = Expr::new(ExprKind::Leaf(Leaf::Var(binding)), Self::span());
            Interpreter::new(&mut NoGroups, Self::span(), StepBudget::full())
                .evaluate(&read)
                .expect_err("no host group holds globals")
        }

        fn refuses_at_the_call(refusal: &Diagnostic) -> bool {
            refusal.code() == Some(&DiagnosticCode::MISSING_HOST_GROUP)
                && refusal
                    .labels()
                    .map(|label| label.span())
                    .eq([Self::span()])
        }
    }

    #[test]
    fn a_built_in_whose_group_the_host_lacks_is_refused_at_the_call() {
        let refusal = RefusalFixture::same_binding_without_names();
        assert!(RefusalFixture::refuses_at_the_call(&refusal));
        assert!(
            refusal.title().contains("`same-binding?`"),
            "{}",
            refusal.title()
        );
    }

    #[test]
    fn a_global_read_without_global_values_is_refused_at_the_read() {
        let refusal = RefusalFixture::global_read_without_globals();
        assert!(RefusalFixture::refuses_at_the_call(&refusal));
        assert!(
            refusal.title().contains("reading a global"),
            "{}",
            refusal.title()
        );
    }
}
