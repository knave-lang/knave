use super::type_memo::TypeMemo;
use crate::ownership::callee_borrows::CalleeBorrows;
use crate::types::{
    BorrowingTypes, CopyRules, Type, TypedExpr, TypedExprKind, TypedProgram, TypedStructExpr,
};

pub struct LoanRules<'p> {
    program: &'p TypedProgram,
    callees: CalleeBorrows<'p>,
    copy: CopyRules<'p>,
    moved: TypeMemo,
    borrowing: TypeMemo,
}

impl<'p> LoanRules<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            callees: CalleeBorrows::new(program),
            copy: CopyRules::new(&program.declarations, &program.generics),
            moved: TypeMemo::default(),
            borrowing: TypeMemo::default(),
        }
    }

    pub fn moves(&self, ty: &Type) -> bool {
        self.moved.answer(ty, |ty| !self.copy.is_copy(ty))
    }

    pub fn callees(&self) -> &CalleeBorrows<'p> {
        &self.callees
    }

    pub fn reads_through_a_stored_reference(&self, read: &TypedExpr) -> bool {
        match &read.kind {
            TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) => {
                self.stores_reference(&owner.ty, &label.name)
                    || self.reads_through_a_stored_reference(owner)
            }
            _ => read.ty.is_reference(),
        }
    }

    fn stores_reference(&self, owner: &Type, field: &str) -> bool {
        self.program
            .declarations
            .types()
            .structs()
            .behind(owner)
            .is_some_and(|found| found.definition.holds_reference_in(field, found.arguments))
    }

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        self.borrowing
            .answer(ty, |ty| BorrowingTypes::of(self.program).keeps_borrow(ty))
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{ExprKind, Leaf, RustTemplate};
    use crate::types::{BorrowedIndex, TypedSource};

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
    fn no_prelude_template_that_indexes_in_place_has_clauses() {
        assert_eq!(
            IndexTemplates::with_clauses(),
            Vec::<String>::new(),
            "the loan rule reads a template without its clause choice, so it would \
             disagree with the emitter on whether this argument is copied out"
        );
    }
}
