use crate::types::TypedExpr;

#[derive(Clone, Default)]
pub struct TreePosition {
    children: Vec<usize>,
}

impl TreePosition {
    pub fn along(path: &[&TypedExpr]) -> Self {
        let children = path
            .windows(2)
            .map(|step| {
                step[0]
                    .children()
                    .into_iter()
                    .position(|candidate| std::ptr::eq(candidate, step[1]))
                    .expect("each node of the path is a child of the one before it")
            })
            .collect();
        Self { children }
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
    use crate::emit::test_program::TestProgram;
    use crate::types::TypedExpr;

    struct PositionFixture {
        program: TestProgram,
    }

    impl PositionFixture {
        const SOURCE: &str = "
            (defn pick (low high) (+ low high))
            (defn main () (local sum (pick 1 (pick 2 3))) (println sum))";

        fn new() -> Self {
            Self {
                program: TestProgram::new(Self::SOURCE),
            }
        }

        fn sum(&self) -> &TypedExpr {
            self.program.last_let_value("main")
        }

        fn last_child(expr: &TypedExpr) -> &TypedExpr {
            expr.children().into_iter().last().expect("a child")
        }

        fn text(&self, expr: &TypedExpr) -> &'static str {
            self.program.text(expr)
        }
    }

    #[test]
    fn finds_a_nested_child_in_a_copy_of_the_tree() {
        let fixture = PositionFixture::new();
        let sum = fixture.sum();
        let outer = PositionFixture::last_child(sum);
        let position = TreePosition::along(&[sum, outer]);
        let mut copy = sum.clone();
        assert_eq!(fixture.text(position.node_mut(&mut copy)), "(pick 2 3)");
    }

    #[test]
    fn a_path_of_one_node_names_the_root() {
        let fixture = PositionFixture::new();
        let sum = fixture.sum();
        let position = TreePosition::along(&[sum]);
        let mut copy = sum.clone();
        assert_eq!(
            fixture.text(position.node_mut(&mut copy)),
            "(pick 1 (pick 2 3))"
        );
    }
}
