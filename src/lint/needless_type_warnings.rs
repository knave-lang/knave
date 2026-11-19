use super::function_declaration::FunctionDeclaration;
use super::needless_type_trial::ByValueRewrites;
use super::problems::LintProblems;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::expand::Program;

pub struct NeedlessTypeWarnings<'p> {
    program: &'p Program,
}

impl<'p> NeedlessTypeWarnings<'p> {
    pub fn new(program: &'p Program) -> Self {
        Self { program }
    }

    pub fn of(
        &self,
        declarations: &[FunctionDeclaration],
        removable: &[BindingId],
        by_value: &ByValueRewrites,
    ) -> Diagnostics {
        let mut warnings = Diagnostics::default();
        for declaration in declarations {
            if removable.contains(&declaration.function) {
                warnings.push(self.warning(declaration, by_value));
            }
        }
        warnings
    }

    fn warning(&self, declaration: &FunctionDeclaration, by_value: &ByValueRewrites) -> Diagnostic {
        let name = self.name(declaration.function);
        match by_value.get(&declaration.function) {
            None => LintProblems::needless_type(&name, declaration.span),
            Some(rewrite) => {
                let parameters: Vec<String> = rewrite
                    .positions
                    .iter()
                    .filter_map(|position| declaration.parameters.get(*position))
                    .map(|parameter| self.name(*parameter))
                    .collect();
                LintProblems::needless_type_by_value(&name, &parameters, declaration.span)
            }
        }
    }

    fn name(&self, binding: BindingId) -> String {
        self.program.bindings.get(binding).name().to_string()
    }
}
