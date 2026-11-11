use crate::types::TypedExpr;

#[derive(Clone, Default)]
pub struct TreePosition {
    children: Vec<usize>,
}

impl TreePosition {
    pub fn enter(&mut self, parent: &TypedExpr, child: &TypedExpr) {
        let index = parent
            .children()
            .into_iter()
            .position(|candidate| std::ptr::eq(candidate, child))
            .expect("the child is one of its parent's children");
        self.children.push(index);
    }

    pub fn leave(&mut self) {
        self.children.pop();
    }

    pub fn node_mut<'e>(&self, root: &'e mut TypedExpr) -> &'e mut TypedExpr {
        self.children.iter().fold(root, |node, index| {
            node.children_mut()
                .into_iter()
                .nth(*index)
                .expect("the position was taken in a tree of the same shape")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TreePosition;
    use crate::types::{TypedBodyItem, TypedExpr, TypedExprKind, TypedProgram, TypedSource};

    struct PositionFixture {
        program: TypedProgram,
    }

    impl PositionFixture {
        const SOURCE: &str = "
            (defn pick (low high) (+ low high))
            (defn main () (println (pick 1 (pick 2 3))))";

        fn new() -> Self {
            Self {
                program: TypedSource::expand(Self::SOURCE).typed(),
            }
        }

        fn printed(&self) -> &TypedExpr {
            let main = self
                .program
                .modules
                .iter()
                .flat_map(|module| &module.definitions)
                .find(|definition| self.program.bindings.get(definition.binding).name() == "main")
                .expect("the fixture defines main");
            let TypedExprKind::Fn(lambda) = &main.value.kind else {
                panic!("main is a function");
            };
            match lambda.body.items.as_slice() {
                [TypedBodyItem::Expr(printed)] => printed,
                _ => panic!("main holds one form"),
            }
        }

        fn last_child(expr: &TypedExpr) -> &TypedExpr {
            expr.children().into_iter().last().expect("a child")
        }

        fn text(expr: &TypedExpr) -> &'static str {
            &Self::SOURCE[expr.span.range()]
        }
    }

    #[test]
    fn finds_a_nested_child_in_a_copy_of_the_tree() {
        let fixture = PositionFixture::new();
        let printed = fixture.printed();
        let outer = PositionFixture::last_child(printed);
        let inner = PositionFixture::last_child(outer);
        let mut position = TreePosition::default();
        position.enter(printed, outer);
        position.enter(outer, inner);
        let mut copy = printed.clone();
        assert_eq!(
            PositionFixture::text(position.node_mut(&mut copy)),
            "(pick 2 3)"
        );
    }

    #[test]
    fn leaving_returns_to_the_parent() {
        let fixture = PositionFixture::new();
        let printed = fixture.printed();
        let outer = PositionFixture::last_child(printed);
        let inner = PositionFixture::last_child(outer);
        let mut position = TreePosition::default();
        position.enter(printed, outer);
        position.enter(outer, inner);
        position.leave();
        let mut copy = printed.clone();
        assert_eq!(
            PositionFixture::text(position.node_mut(&mut copy)),
            "(pick 1 (pick 2 3))"
        );
    }
}
