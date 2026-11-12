use super::body_lists::BodyLists;
use super::shapes::{Shape, ShapeCatalog};
use super::syntax_tree::{Entry, ListNode};
use super::written_gap::WrittenGap;

#[derive(Default)]
pub struct MethodEntries {
    methods: Vec<usize>,
    separated: Vec<usize>,
}

impl MethodEntries {
    pub fn of(list: &ListNode, shape: Option<Shape>, shapes: &ShapeCatalog) -> Self {
        let Some(shape) = shape.filter(|shape| shape.body_lists == BodyLists::Methods) else {
            return Self::default();
        };
        let methods: Vec<usize> = (shape.header_length + 1..list.entries.len())
            .filter(|entry| Self::is_method(&list.entries[*entry], shapes))
            .collect();
        let separated = methods
            .windows(2)
            .filter(|pair| Self::needs_blank_line(list, pair[0], pair[1]))
            .map(|pair| pair[1])
            .collect();
        Self { methods, separated }
    }

    pub fn entries(&self) -> &[usize] {
        &self.methods
    }

    pub fn takes_blank_line(&self, entry: usize) -> bool {
        self.separated.contains(&entry)
    }

    fn needs_blank_line(list: &ListNode, earlier: usize, later: usize) -> bool {
        let neighbors = earlier + 1 == later;
        let either_has_body = [earlier, later]
            .iter()
            .any(|entry| Self::has_body(&list.entries[*entry]));
        let written_blank_line = list.entries[later].gap_before() == WrittenGap::BlankLine;
        neighbors && either_has_body && !written_blank_line
    }

    fn is_method(entry: &Entry, shapes: &ShapeCatalog) -> bool {
        entry.node.as_list().is_some_and(|list| {
            let unshaped_name = list
                .head_name()
                .is_some_and(|name| shapes.of(name).is_none());
            let signature_is_list = list
                .entries
                .get(1)
                .is_some_and(|signature| signature.node.as_list().is_some());
            unshaped_name && signature_is_list
        })
    }

    fn has_body(entry: &Entry) -> bool {
        let header_end = Shape::METHOD.header_length + 1;
        entry
            .node
            .as_list()
            .is_some_and(|list| list.entries.len() > header_end)
    }
}

#[cfg(test)]
mod tests {
    use super::MethodEntries;
    use crate::format::shapes::{ShapeCatalog, ShapeCatalogFixture};
    use crate::format::syntax_tree::SyntaxTree;

    struct MethodsFixture {
        shapes: ShapeCatalog,
    }

    impl MethodsFixture {
        const SHAPES: &'static str = "(header-shape (impl trait type) :methods)
             (header-shape (trait name) :methods)
             (header-shape (plain trait type))
             (header-shape (where parameters))";

        fn new() -> Self {
            Self {
                shapes: ShapeCatalogFixture::stated(Self::SHAPES),
            }
        }

        fn methods(&self, text: &str) -> MethodEntries {
            let tree = SyntaxTree::build(text);
            let list = tree.entries[0].node.as_list().expect("the text is a list");
            let shape = list.head_name().and_then(|name| self.shapes.of(name));
            MethodEntries::of(list, shape, &self.shapes)
        }

        fn method_entries(&self, text: &str) -> Vec<usize> {
            self.methods(text).entries().to_vec()
        }

        fn takes_blank_line(&self, text: &str, entry: usize) -> bool {
            self.methods(text).takes_blank_line(entry)
        }
    }

    #[test]
    fn each_list_of_a_name_and_a_list_after_the_header_is_a_method() {
        let fixture = MethodsFixture::new();
        assert_eq!(
            fixture.method_entries("(impl t x (a (s) 1) (b () 2))"),
            vec![3, 4]
        );
    }

    #[test]
    fn a_list_in_the_header_is_not_a_method() {
        let fixture = MethodsFixture::new();
        assert_eq!(
            fixture.method_entries("(impl t (rank (s) 1) (m (s) 1))"),
            vec![3]
        );
    }

    #[test]
    fn a_list_whose_second_form_is_not_a_list_is_not_a_method() {
        let fixture = MethodsFixture::new();
        assert_eq!(
            fixture.method_entries("(impl t x (type item a) (a (s) 1) item)"),
            vec![4]
        );
    }

    #[test]
    fn a_list_headed_by_a_shaped_form_is_not_a_method() {
        let fixture = MethodsFixture::new();
        assert!(
            fixture
                .method_entries("(impl t x (where (a) b))")
                .is_empty()
        );
    }

    #[test]
    fn a_list_without_a_head_name_is_not_a_method() {
        let fixture = MethodsFixture::new();
        assert!(
            fixture
                .method_entries("(impl t x (\"a\" (s) 1))")
                .is_empty()
        );
    }

    #[test]
    fn a_form_without_the_marker_has_no_methods() {
        let fixture = MethodsFixture::new();
        assert!(fixture.method_entries("(plain t x (a (s) 1))").is_empty());
    }

    #[test]
    fn a_blank_line_separates_two_methods_with_bodies() {
        let fixture = MethodsFixture::new();
        let text = "(impl t x (a (s) 1) (b (s) 2))";
        assert!(fixture.takes_blank_line(text, 4));
        assert!(!fixture.takes_blank_line(text, 3));
    }

    #[test]
    fn no_blank_line_separates_two_signatures() {
        let fixture = MethodsFixture::new();
        assert!(!fixture.takes_blank_line("(trait t (a (fn () i64)) (b (fn () i64)))", 3));
    }

    #[test]
    fn a_blank_line_separates_a_signature_from_a_later_body() {
        let fixture = MethodsFixture::new();
        assert!(fixture.takes_blank_line("(trait t (a (fn () i64)) (b (fn () i64) 1))", 3));
    }

    #[test]
    fn a_blank_line_separates_a_body_from_a_later_signature() {
        let fixture = MethodsFixture::new();
        assert!(fixture.takes_blank_line("(trait t (b (fn () i64) 1) (a (fn () i64)))", 3));
    }

    #[test]
    fn no_blank_line_separates_methods_with_another_entry_between() {
        let fixture = MethodsFixture::new();
        assert!(!fixture.takes_blank_line("(impl t x (a (s) 1) (type item a) (b (s) 2))", 5));
    }

    #[test]
    fn a_written_blank_line_takes_no_second_one() {
        let fixture = MethodsFixture::new();
        assert!(!fixture.takes_blank_line("(impl t x (a (s) 1)\n\n (b (s) 2))", 4));
    }

    #[test]
    fn a_written_blank_line_above_a_comment_takes_no_second_one() {
        let fixture = MethodsFixture::new();
        let text = "(impl t x (a (s) 1)\n\n ;; b\n (b (s) 2))";
        assert!(!fixture.takes_blank_line(text, 4));
    }

    #[test]
    fn a_blank_line_below_a_comment_leaves_room_above_it() {
        let fixture = MethodsFixture::new();
        let text = "(impl t x (a (s) 1)\n ;; b\n\n (b (s) 2))";
        assert!(fixture.takes_blank_line(text, 4));
    }
}
