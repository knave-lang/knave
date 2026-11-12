use rustc_hash::FxHashMap;

use super::list_plan::ListPlan;
use super::list_role::ListRole;
use super::shapes::{Shape, ShapeCatalog};
use super::syntax_tree::ListNode;

#[derive(Default)]
pub struct ListRoles {
    roles: FxHashMap<usize, ListRole>,
}

impl ListRoles {
    pub fn plan(&mut self, list: &ListNode, shapes: &ShapeCatalog) -> ListPlan {
        let plan = self.fresh_plan(list, shapes);
        self.mark_nested_lists(list, &plan);
        plan
    }

    pub fn is_arm(&self, list: &ListNode) -> bool {
        self.roles.get(&list.id) == Some(&ListRole::Arm)
    }

    fn fresh_plan(&self, list: &ListNode, shapes: &ShapeCatalog) -> ListPlan {
        match self.roles.get(&list.id) {
            Some(ListRole::BindingList) => ListPlan::binding_list(list),
            Some(ListRole::Method) => ListPlan::new(list, Some(Shape::METHOD), shapes),
            Some(ListRole::Arm) | None => {
                let shape = list.head_name().and_then(|name| shapes.of(name));
                ListPlan::new(list, shape, shapes)
            }
        }
    }

    fn mark_nested_lists(&mut self, list: &ListNode, plan: &ListPlan) {
        for (entry, role) in plan.nested_roles() {
            if let Some(nested) = list
                .entries
                .get(entry)
                .and_then(|entry| entry.node.as_list())
            {
                self.roles.insert(nested.id, role);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ListRoles;
    use crate::format::shapes::{ShapeCatalog, ShapeCatalogFixture};
    use crate::format::syntax_tree::{ListNode, SyntaxTree};

    struct RolesFixture {
        shapes: ShapeCatalog,
        tree: SyntaxTree,
    }

    impl RolesFixture {
        const SHAPES: &'static str =
            "(header-shape (impl trait type) :methods) (header-shape (match value) :arms)";

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

        fn entry(&self, entry: usize) -> &ListNode {
            self.list().entries[entry]
                .node
                .as_list()
                .expect("the entry is a list")
        }

        fn allows_one_line_after_its_parent(&self, entry: usize) -> bool {
            let mut roles = ListRoles::default();
            roles.plan(self.list(), &self.shapes);
            roles
                .plan(self.entry(entry), &self.shapes)
                .allows_one_line()
        }

        fn arms_after_planning_the_parent(&self) -> Vec<bool> {
            let mut roles = ListRoles::default();
            roles.plan(self.list(), &self.shapes);
            self.list()
                .entries
                .iter()
                .filter_map(|entry| entry.node.as_list())
                .map(|nested| roles.is_arm(nested))
                .collect()
        }

        fn allows_one_line_alone(&self, entry: usize) -> bool {
            let mut roles = ListRoles::default();
            roles
                .plan(self.entry(entry), &self.shapes)
                .allows_one_line()
        }
    }

    #[test]
    fn a_method_with_two_body_forms_never_stays_on_one_line() {
        let fixture = RolesFixture::new("(impl t x (m (s) (print s) s))");
        assert!(!fixture.allows_one_line_after_its_parent(3));
    }

    #[test]
    fn a_method_with_one_body_form_may_stay_on_one_line() {
        let fixture = RolesFixture::new("(impl t x (m (s) s))");
        assert!(fixture.allows_one_line_after_its_parent(3));
    }

    #[test]
    fn a_list_after_the_header_of_a_form_with_arms_is_an_arm() {
        let fixture = RolesFixture::new("(match v (a b) (c d))");
        assert_eq!(fixture.arms_after_planning_the_parent(), vec![true, true]);
    }

    #[test]
    fn a_header_list_of_a_form_with_arms_is_no_arm() {
        let fixture = RolesFixture::new("(match (v w) (a b))");
        assert_eq!(fixture.arms_after_planning_the_parent(), vec![false, true]);
    }

    #[test]
    fn a_keyword_value_is_no_arm() {
        let fixture = RolesFixture::new("(match v :key (a b) (c d))");
        assert_eq!(fixture.arms_after_planning_the_parent(), vec![false, true]);
    }

    #[test]
    fn a_list_outside_a_method_position_lays_out_as_a_call() {
        let fixture = RolesFixture::new("(impl t x (m (s) (print s) s))");
        assert!(fixture.allows_one_line_alone(3));
    }
}
