use super::dereferenced_placeholders::DereferencedPlaceholders;
use crate::types::{
    BorrowedIndex, BorrowingTypes, CopiedOut, CopyRules, TemplateTies, Type, TypedArgument,
    TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
};

pub struct LoanRules<'p> {
    program: &'p TypedProgram,
    copy: CopyRules<'p>,
}

impl<'p> LoanRules<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub fn moves(&self, ty: &Type) -> bool {
        !self.copy.is_copy(ty)
    }

    pub fn ties(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        self.program
            .template_borrows
            .read_template(callee, None, |_, ties| ties.cloned())
    }

    pub fn dereferences_every_use(&self, callee: &TypedExpr, position: usize) -> bool {
        !self.chooses_a_clause(callee)
            && self
                .program
                .template_borrows
                .read_template(callee, false, |parsed, _| {
                    DereferencedPlaceholders::new(parsed).dereference_every_use(position)
                })
    }

    fn chooses_a_clause(&self, callee: &TypedExpr) -> bool {
        matches!(&callee.kind, TypedExprKind::Leaf(TypedLeaf::Global(binding))
            if self.program.template_clauses.chooses(*binding))
    }

    pub fn copies_out(&self, argument: &TypedArgument) -> bool {
        CopiedOut::of(argument, |value| self.indexes_in_place(value)).is_some()
    }

    fn indexes_in_place(&self, value: &TypedExpr) -> bool {
        let TypedExprKind::Call(callee, _) = &value.kind else {
            return false;
        };
        self.program
            .template_borrows
            .read_template(callee, false, |parsed, _| {
                BorrowedIndex::place(parsed.text()).is_some()
            })
    }

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        BorrowingTypes::of(self.program).keeps_borrow(ty)
    }
}

#[cfg(test)]
mod tests {
    use super::LoanRules;
    use crate::core::{ExprKind, Leaf, RustTemplate};
    use crate::ownership::lambda_source::LambdaSource;
    use crate::types::{BorrowedIndex, TypedExpr, TypedExprKind, TypedSource};

    struct ClauseCalls;

    impl ClauseCalls {
        const SOURCE: &str = "(def size (rust \"{0}.len()\" (where (a) (fn ((ref a)) usize))))\n\
            (def sized (rust \"{0}.len()\" (where (a) (fn ((ref a)) usize)) (str \"{0}.len()\")))\n\
            (defn main ()\n  (local plain (size \"ab\"))\n  (local chosen (sized \"ab\"))\n  (println plain chosen))";

        fn callee<'s>(source: &'s LambdaSource, name: &str) -> &'s TypedExpr {
            match &source.bound(name).kind {
                TypedExprKind::Call(callee, _) => callee,
                _ => panic!("`{name}` is bound to a call"),
            }
        }
    }

    struct IndexTemplates;

    impl IndexTemplates {
        fn with_clauses() -> Vec<String> {
            let source = TypedSource::expand("(defn main () (println 1))");
            let program = source.program();
            program
                .modules
                .iter()
                .flat_map(|module| module.definitions())
                .filter_map(|(binding, value)| match value.kind() {
                    ExprKind::Leaf(Leaf::Rust(template)) => Some((binding, template)),
                    _ => None,
                })
                .filter(|(_, template)| Self::indexes_with_clauses(template))
                .map(|(binding, _)| program.bindings.get(binding).name().to_string())
                .collect()
        }

        fn indexes_with_clauses(template: &RustTemplate) -> bool {
            let texts = template.clauses.iter().map(|clause| &clause.template);
            !template.clauses.is_empty()
                && texts
                    .chain([&template.template])
                    .any(|text| BorrowedIndex::place(text).is_some())
        }
    }

    #[test]
    fn a_template_that_chooses_a_clause_dereferences_no_argument() {
        let source = LambdaSource::new(ClauseCalls::SOURCE);
        let rules = LoanRules::of(source.program());
        assert!(rules.dereferences_every_use(ClauseCalls::callee(&source, "plain"), 0));
        assert!(!rules.dereferences_every_use(ClauseCalls::callee(&source, "chosen"), 0));
    }

    #[test]
    fn no_prelude_template_that_indexes_in_place_has_clauses() {
        assert_eq!(
            IndexTemplates::with_clauses(),
            Vec::<String>::new(),
            "the loan rule reads a template without its clause choice, so it would \
             disagree with the emitter on whether this argument is copied out"
        );
    }
}
