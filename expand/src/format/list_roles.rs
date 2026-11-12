use rustc_hash::FxHashSet;

use super::list_plan::ListPlan;
use super::shapes::ShapeCatalog;
use super::syntax_tree::ListNode;

#[derive(Default)]
pub struct ListRoles {
    binding_lists: FxHashSet<usize>,
    methods: FxHashSet<usize>,
}

impl ListRoles {
    pub fn plan(&mut self, list: &ListNode, shapes: &ShapeCatalog) -> ListPlan {
        let plan = self.fresh_plan(list, shapes);
        self.mark_entries(list, &plan);
        plan
    }

    fn fresh_plan(&self, list: &ListNode, shapes: &ShapeCatalog) -> ListPlan {
        if self.binding_lists.contains(&list.id) {
            ListPlan::binding_list(list)
        } else if self.methods.contains(&list.id) {
            ListPlan::method(list)
        } else {
            ListPlan::new(list, shapes)
        }
    }

    fn mark_entries(&mut self, list: &ListNode, plan: &ListPlan) {
        let binding_list = plan
            .binding_list_entry
            .and_then(|entry| Self::list_at(list, entry));
        self.binding_lists
            .extend(binding_list.map(|binding_list| binding_list.id));
        let methods = plan.methods.entries().iter();
        self.methods.extend(
            methods
                .filter_map(|entry| Self::list_at(list, *entry))
                .map(|method| method.id),
        );
    }

    fn list_at(list: &ListNode, entry: usize) -> Option<&ListNode> {
        list.entries.get(entry)?.node.as_list()
    }
}

#[cfg(test)]
mod tests {
    use super::ListRoles;
    use crate::format::shapes::{ShapeCatalog, ShapeStatement};
    use crate::format::syntax_tree::{ListNode, SyntaxTree};
    use crate::reader::Reader;
    use crate::source::SourceMap;

    struct RolesFixture {
        shapes: ShapeCatalog,
        tree: SyntaxTree,
    }

    impl RolesFixture {
        const SHAPE: &'static str = "(header-shape (impl trait type) :methods)";

        fn new(text: &str) -> Self {
            let mut sources = SourceMap::default();
            let source = sources.add("shape", Self::SHAPE);
            let form = Reader::new(sources.file(source))
                .read()
                .unwrap_or_else(|_| panic!("the shape reads"))
                .remove(0);
            let mut shapes = ShapeCatalog::default();
            shapes.add(&ShapeStatement::of(&form).expect("the shape states a shape"));
            Self {
                shapes,
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
    fn a_list_outside_a_method_position_lays_out_as_a_call() {
        let fixture = RolesFixture::new("(impl t x (m (s) (print s) s))");
        assert!(fixture.allows_one_line_alone(3));
    }
}
