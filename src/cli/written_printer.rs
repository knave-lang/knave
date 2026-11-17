use crate::core::{BindingId, Expr, ExprKind, Leaf};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, ResultPrinter};

pub struct WrittenPrinter {
    name: &'static str,
    source: &'static str,
    bound: &'static str,
}

impl WrittenPrinter {
    pub fn new(name: &'static str, source: &'static str, bound: &'static str) -> Self {
        Self {
            name,
            source,
            bound,
        }
    }

    pub fn build(
        &self,
        session: &mut ExpansionSession<'_>,
        text: &str,
    ) -> Result<ResultPrinter, Diagnostics> {
        let bound_holder = self.bound_holder(session)?;
        let source = session.add_source(self.source, text);
        let printer = session.prelude_function(self.name, source)?;
        Ok(ResultPrinter::through(printer, bound_holder))
    }

    fn bound_holder(&self, session: &mut ExpansionSession<'_>) -> Result<BindingId, Diagnostics> {
        let source = session.add_source(self.source, self.bound);
        let form = session.read(source)?.remove(0);
        let expanded = session.expression(form)?;
        Ok(Self::binding_of(&expanded).expect("a printer's bound term expands to a binding"))
    }

    fn binding_of(expanded: &Expr) -> Option<BindingId> {
        match expanded.kind() {
            ExprKind::Leaf(Leaf::Var(binding)) => Some(*binding),
            _ => None,
        }
    }
}
