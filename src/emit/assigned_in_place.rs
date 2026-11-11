use super::lift_placement::LiftPlacement;
use super::lifted_temporary::LiftedTemporary;
use super::rust_leaf::RustLeaf;
use super::temporary_scope::TemporaryScope;
use super::tree_position::TreePosition;
use crate::types::{Borrow, ClosureKind, Lifetime, Type, TypedArgument, TypedExpr, TypedExprKind};

pub struct AssignedInPlace {
    site: TreePosition,
    lend: LendSite,
}

#[derive(Clone, Copy)]
enum LendSite {
    Argument,
    WrittenBorrow,
}

impl AssignedInPlace {
    const BLOCK: &'static str = "{{ OWNER = {0}; &OWNER }}";
    const OWNER_SLOT: &'static str = "OWNER";

    pub fn of_argument(owner: TreePosition) -> Self {
        Self {
            site: owner,
            lend: LendSite::Argument,
        }
    }

    pub fn of_written_borrow(borrow: TreePosition) -> Self {
        Self {
            site: borrow,
            lend: LendSite::WrittenBorrow,
        }
    }

    fn assigned(&self, name: &str, site: TypedExpr) -> TypedExpr {
        let (owned, reference) = self.owned_and_reference(site);
        let span = owned.span;
        let block = Self::BLOCK.replace(Self::OWNER_SLOT, name);
        let callee_type = Type::Function(
            ClosureKind::Fn,
            vec![owned.ty.clone()],
            Box::new(reference.clone()),
        );
        let argument = TypedArgument {
            value: owned,
            borrow: Borrow::None,
        };
        TypedExpr {
            kind: TypedExprKind::Call(
                Box::new(RustLeaf::new(block).typed(callee_type, span)),
                vec![argument],
            ),
            ty: reference,
            span,
        }
    }

    fn owned_and_reference(&self, site: TypedExpr) -> (TypedExpr, Type) {
        match (self.lend, site.kind) {
            (LendSite::WrittenBorrow, TypedExprKind::Borrow(_, owned)) => (*owned, site.ty),
            (_, kind) => {
                let owned = TypedExpr { kind, ..site };
                let reference = Type::Ref(Lifetime::Elided, Box::new(owned.ty.clone()));
                (owned, reference)
            }
        }
    }
}

impl LiftPlacement for AssignedInPlace {
    fn inner_scope(&self) -> TemporaryScope {
        TemporaryScope::Statement
    }

    fn settles_inside(&self) -> bool {
        false
    }

    fn take(&self, root: &mut TypedExpr, name: String) -> LiftedTemporary {
        let node = self.site.node_mut(root);
        let placeholder = RustLeaf::new(name.as_str()).typed(node.ty.clone(), node.span);
        let site = std::mem::replace(node, placeholder);
        *node = self.assigned(&name, site);
        LiftedTemporary::declared(name)
    }
}

#[cfg(test)]
mod tests {
    use super::AssignedInPlace;
    use crate::emit::fragment::Code;
    use crate::emit::lift_placement::LiftPlacement;
    use crate::emit::test_program::TestProgram;
    use crate::emit::tree_position::TreePosition;
    use crate::types::{TypedExpr, TypedExprKind, TypedLeaf};

    struct AssignedFixture {
        program: TestProgram,
    }

    impl AssignedFixture {
        const SOURCE: &str = "
            (defstruct page :title string)
            (type first-letter (fn ((ref str)) (ref str)))
            (defn first-letter (text) (subs text 0 1))
            (type title-of (fn ((ref page)) (ref str)))
            (defn title-of (page) (as-str (:title page)))
            (defn lent (name)
              (local letter (first-letter (as-str (str name \"-x\"))))
              (len letter))
            (defn written (name)
              (local letter (title-of (ref (->page :title (str name)))))
              (len letter))
            (defn main ())";

        fn new() -> Self {
            Self {
                program: TestProgram::new(Self::SOURCE),
            }
        }

        fn lent_owner(value: &TypedExpr) -> TreePosition {
            let lent = &TestProgram::arguments(value)[0].value;
            let owner = &TestProgram::arguments(lent)[0].value;
            TreePosition::along(&[value, lent, owner])
        }

        fn written_borrow(value: &TypedExpr) -> TreePosition {
            let borrow = &TestProgram::arguments(value)[0].value;
            TreePosition::along(&[value, borrow])
        }

        fn assignment(&self, assigned: &TypedExpr) -> (String, &'static str) {
            let TypedExprKind::Leaf(TypedLeaf::Rust(template)) =
                &TestProgram::callee(assigned).kind
            else {
                panic!("the value is assigned through a template");
            };
            let owned = &TestProgram::arguments(assigned)[0].value;
            (template.template.to_string(), self.program.text(owned))
        }
    }

    #[test]
    fn assigns_a_lent_argument_where_it_is_written() {
        let fixture = AssignedFixture::new();
        let value = fixture.program.last_let_value("lent");
        let position = AssignedFixture::lent_owner(value);
        let mut rewritten = value.clone();
        let temporary = AssignedInPlace::of_argument(position.clone())
            .take(&mut rewritten, "owner".to_string());
        assert_eq!(
            Code::text_of(&temporary.statement(|_| Code::atom(""))),
            "let owner;"
        );
        let assigned = position.node_mut(&mut rewritten);
        assert!(assigned.ty.is_reference());
        assert_eq!(
            fixture.assignment(assigned),
            ("{{ owner = {0}; &owner }}".to_string(), "(str name \"-x\")")
        );
    }

    #[test]
    fn assigns_a_written_borrow_in_place_of_the_borrow() {
        let fixture = AssignedFixture::new();
        let value = fixture.program.last_let_value("written");
        let position = AssignedFixture::written_borrow(value);
        let borrow_type = position.node_mut(&mut value.clone()).ty.clone();
        let mut rewritten = value.clone();
        AssignedInPlace::of_written_borrow(position.clone())
            .take(&mut rewritten, "owner".to_string());
        let assigned = position.node_mut(&mut rewritten);
        assert_eq!(assigned.ty, borrow_type);
        assert_eq!(fixture.assignment(assigned).1, "(->page :title (str name))");
    }
}
