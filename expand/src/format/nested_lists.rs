use super::body_lists::BodyLists;
use super::list_role::ListRole;
use super::method_entries::MethodEntries;
use super::shapes::{Shape, ShapeCatalog};
use super::syntax_tree::ListNode;
use super::unit::Unit;

#[derive(Clone, Default)]
pub struct NestedLists {
    binding_list_entry: Option<usize>,
    first_arm_entry: Option<usize>,
    methods: MethodEntries,
}

impl NestedLists {
    pub fn of(list: &ListNode, shape: Option<Shape>, shapes: &ShapeCatalog) -> Self {
        Self {
            binding_list_entry: shape
                .filter(|shape| shape.bindings)
                .map(|shape| shape.header_length),
            first_arm_entry: shape
                .filter(|shape| shape.body_lists == BodyLists::Arms)
                .map(|shape| shape.header_length + 1),
            methods: MethodEntries::of(list, shape, shapes),
        }
    }

    pub fn roles<'n>(&'n self, units: &'n [Unit]) -> impl Iterator<Item = (usize, ListRole)> + 'n {
        let binding_list = self
            .binding_list_entry
            .map(|entry| (entry, ListRole::BindingList));
        let methods = self
            .methods
            .entries()
            .iter()
            .map(|entry| (*entry, ListRole::Method));
        let arms = self.arm_entries(units).map(|entry| (entry, ListRole::Arm));
        binding_list.into_iter().chain(methods).chain(arms)
    }

    pub fn takes_blank_line_before(&self, entry: usize) -> bool {
        self.methods.takes_blank_line(entry)
    }

    fn arm_entries<'n>(&self, units: &'n [Unit]) -> impl Iterator<Item = usize> + 'n {
        let first_arm = self.first_arm_entry;
        units.iter().filter_map(move |unit| match unit {
            Unit::Single(entry) if first_arm.is_some_and(|first| *entry >= first) => Some(*entry),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NestedLists;
    use crate::format::list_plan::ListPlan;
    use crate::format::list_role::ListRole;
    use crate::format::shape_catalog_fixture::ShapeCatalogFixture;
    use crate::format::shapes::{Shape, ShapeCatalog};
    use crate::format::syntax_tree::{ListNode, SyntaxTree};

    struct NestedFixture {
        shapes: ShapeCatalog,
        tree: SyntaxTree,
    }

    impl NestedFixture {
        const SHAPES: &'static str = "(header-shape (impl trait type) :methods)
             (header-shape (match value) :arms)
             (header-shape (let bindings) :bindings)";

        fn new(text: &str) -> Self {
            Self {
                shapes: ShapeCatalogFixture::stated(Self::SHAPES),
                tree: SyntaxTree::build(text),
            }
        }

        fn list(&self) -> &ListNode {
            self.tree.entries[0]
                .node
                .as_list()
                .expect("the text is a list")
        }

        fn shape(&self) -> Option<Shape> {
            self.list()
                .head_name()
                .and_then(|name| self.shapes.of(name))
        }

        fn nested(&self) -> NestedLists {
            NestedLists::of(self.list(), self.shape(), &self.shapes)
        }

        fn roles(&self) -> Vec<(usize, ListRole)> {
            let nested = self.nested();
            let plan = ListPlan::new(self.list(), self.shape(), nested.clone());
            nested.roles(&plan.units).collect()
        }
    }

    #[test]
    fn the_last_header_form_of_a_form_with_bindings_is_a_binding_list() {
        let fixture = NestedFixture::new("(let (x 1) x)");
        assert_eq!(fixture.roles(), vec![(1, ListRole::BindingList)]);
    }

    #[test]
    fn each_method_of_a_form_with_methods_has_the_method_role() {
        let fixture = NestedFixture::new("(impl t x (a (s) 1) (b (s) 2))");
        assert_eq!(
            fixture.roles(),
            vec![(3, ListRole::Method), (4, ListRole::Method)]
        );
    }

    #[test]
    fn each_form_after_the_header_of_a_form_with_arms_is_an_arm() {
        let fixture = NestedFixture::new("(match v (a b) :key (c d) (e f))");
        assert_eq!(
            fixture.roles(),
            vec![(2, ListRole::Arm), (5, ListRole::Arm)]
        );
    }

    #[test]
    fn a_call_gives_its_lists_no_role() {
        let fixture = NestedFixture::new("(call (a b) (c d))");
        assert!(fixture.roles().is_empty());
    }

    #[test]
    fn a_method_after_another_with_a_body_takes_a_blank_line() {
        let fixture = NestedFixture::new("(impl t x (a (s) 1) (b (s) 2))");
        assert!(fixture.nested().takes_blank_line_before(4));
    }
}
