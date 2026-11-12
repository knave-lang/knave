use rustc_hash::FxHashMap;

use super::body_lists::BodyLists;
use super::docstring_place::DocstringPlace;
use super::one_line_rule::OneLineRule;
use super::syntax_tree::ListNode;
use crate::core::Keyword;
use crate::reader::Form;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub header_length: usize,
    pub docstring: DocstringPlace,
    pub one_line: OneLineRule,
    pub one_per_line: bool,
    pub body_lists: BodyLists,
    pub bindings: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeStatement {
    pub form_name: String,
    pub shape: Shape,
}

#[derive(Clone, Default)]
pub struct ShapeCatalog {
    shapes: FxHashMap<String, Shape>,
}

impl ShapeStatement {
    pub const STATING_FORM: &'static str = "header-shape";
    pub const DOCSTRING_MARKER: &'static str = "docstring";
    pub const DOCSTRING_ONLY_MARKER: &'static str = "docstring-only";
    pub const ONE_PER_LINE_MARKER: &'static str = "one-per-line";
    pub const ARMS_MARKER: &'static str = "arms";
    pub const BINDINGS_MARKER: &'static str = "bindings";
    pub const METHODS_MARKER: &'static str = "methods";

    pub fn of(form: &Form) -> Option<Self> {
        let [head, stated, markers @ ..] = form.as_list()? else {
            return None;
        };
        (head.as_symbol()?.name() == Self::STATING_FORM).then_some(())?;
        let [form_name, header @ ..] = stated.as_list()? else {
            return None;
        };
        Some(Self {
            form_name: form_name.as_symbol()?.name().to_string(),
            shape: Shape {
                header_length: header.len(),
                docstring: Self::docstring_place(markers),
                one_line: Self::one_line_rule(markers),
                one_per_line: Self::has_marker(markers, Self::ONE_PER_LINE_MARKER),
                body_lists: Self::body_lists(markers),
                bindings: Self::has_marker(markers, Self::BINDINGS_MARKER),
            },
        })
    }

    fn docstring_place(markers: &[Form]) -> DocstringPlace {
        if Self::has_marker(markers, Self::DOCSTRING_MARKER) {
            DocstringPlace::BeforeBody
        } else if Self::has_marker(markers, Self::DOCSTRING_ONLY_MARKER) {
            DocstringPlace::Last
        } else {
            DocstringPlace::Absent
        }
    }

    fn one_line_rule(markers: &[Form]) -> OneLineRule {
        if Self::has_marker(markers, Self::DOCSTRING_MARKER) {
            OneLineRule::OneBodyForm
        } else {
            OneLineRule::AnyBody
        }
    }

    fn body_lists(markers: &[Form]) -> BodyLists {
        if Self::has_marker(markers, Self::ARMS_MARKER) {
            BodyLists::Arms
        } else if Self::has_marker(markers, Self::METHODS_MARKER) {
            BodyLists::Methods
        } else {
            BodyLists::Forms
        }
    }

    fn has_marker(markers: &[Form], marker: &str) -> bool {
        markers.iter().any(|form| {
            form.as_symbol()
                .and_then(|symbol| Keyword::field_name(symbol.name()))
                == Some(marker)
        })
    }
}

impl ShapeCatalog {
    pub fn add(&mut self, statement: &ShapeStatement) {
        self.shapes
            .insert(statement.form_name.clone(), statement.shape);
    }

    pub fn of(&self, form_name: &str) -> Option<Shape> {
        self.shapes.get(form_name).copied()
    }

    pub fn docstring_entry(&self, list: &ListNode) -> Option<usize> {
        self.of(list.head_name()?)?.docstring_entry(list)
    }

    pub fn docstring_position(&self, form_name: &str, item_count: usize) -> Option<usize> {
        self.of(form_name)?.docstring_position(item_count)
    }
}

impl Shape {
    pub const METHOD: Shape = Shape {
        header_length: 1,
        docstring: DocstringPlace::Absent,
        one_line: OneLineRule::OneBodyForm,
        one_per_line: false,
        body_lists: BodyLists::Forms,
        bindings: false,
    };

    pub fn allows_one_line(self, item_count: usize) -> bool {
        self.one_line.allows(self.header_length, item_count)
    }

    pub fn docstring_entry(self, list: &ListNode) -> Option<usize> {
        let position = self.docstring_position(list.entries.len())?;
        let is_string = list.entries.get(position)?.node.as_atom()?.is_string();
        is_string.then_some(position)
    }

    fn docstring_position(self, item_count: usize) -> Option<usize> {
        self.docstring.position(self.header_length, item_count)
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyLists, DocstringPlace, OneLineRule, Shape, ShapeStatement};
    use crate::reader::{Form, Reader};
    use crate::source::SourceMap;

    struct StatementFixture;

    impl StatementFixture {
        fn statement(text: &str) -> Option<ShapeStatement> {
            ShapeStatement::of(&Self::form(text))
        }

        fn form(text: &str) -> Form {
            let mut sources = SourceMap::default();
            let source = sources.add("shape", text);
            Reader::new(sources.file(source))
                .read()
                .unwrap_or_else(|_| panic!("the statement reads"))
                .remove(0)
        }
    }

    #[test]
    fn a_statement_counts_the_forms_after_the_form_name() {
        let statement = StatementFixture::statement("(header-shape (impl trait type))");
        assert_eq!(
            statement.map(|statement| statement.shape),
            Some(Shape {
                header_length: 2,
                docstring: DocstringPlace::Absent,
                one_line: OneLineRule::AnyBody,
                one_per_line: false,
                body_lists: BodyLists::Forms,
                bindings: false
            })
        );
    }

    #[test]
    fn a_marker_after_the_shape_states_a_docstring() {
        let statement =
            StatementFixture::statement("(header-shape (defn name parameters) :docstring)");
        assert!(
            statement
                .is_some_and(|statement| statement.shape.docstring == DocstringPlace::BeforeBody)
        );
    }

    #[test]
    fn a_docstring_marker_keeps_a_call_on_one_line_only_with_one_body_form() {
        let statement =
            StatementFixture::statement("(header-shape (defn name parameters) :docstring)");
        assert!(
            statement.is_some_and(|statement| statement.shape.one_line == OneLineRule::OneBodyForm)
        );
    }

    #[test]
    fn a_method_keeps_its_name_and_signature_as_its_header() {
        assert_eq!(
            (
                Shape::METHOD.allows_one_line(3),
                Shape::METHOD.allows_one_line(4)
            ),
            (true, false)
        );
    }

    #[test]
    fn a_docstring_only_marker_states_a_docstring_that_ends_the_form() {
        let statement =
            StatementFixture::statement("(header-shape (document header) :docstring-only)");
        let shape = statement.expect("the statement states a shape").shape;
        assert_eq!(
            (shape.docstring_position(3), shape.docstring_position(4)),
            (Some(2), None)
        );
    }

    #[test]
    fn a_docstring_marker_states_a_docstring_that_another_form_follows() {
        let statement = StatementFixture::statement("(header-shape (def name) :docstring)");
        let shape = statement.expect("the statement states a shape").shape;
        assert_eq!(
            (shape.docstring_position(3), shape.docstring_position(4)),
            (None, Some(2))
        );
    }

    #[test]
    fn a_one_per_line_marker_states_no_docstring() {
        let statement = StatementFixture::statement("(header-shape (enum name) :one-per-line)");
        assert_eq!(
            statement.map(|statement| statement.shape),
            Some(Shape {
                header_length: 1,
                docstring: DocstringPlace::Absent,
                one_line: OneLineRule::AnyBody,
                one_per_line: true,
                body_lists: BodyLists::Forms,
                bindings: false
            })
        );
    }

    #[test]
    fn an_arms_marker_states_arms_alone() {
        let statement = StatementFixture::statement("(header-shape (match value) :arms)");
        assert_eq!(
            statement.map(|statement| statement.shape),
            Some(Shape {
                header_length: 1,
                docstring: DocstringPlace::Absent,
                one_line: OneLineRule::AnyBody,
                one_per_line: false,
                body_lists: BodyLists::Arms,
                bindings: false
            })
        );
    }

    #[test]
    fn a_bindings_marker_states_a_binding_list() {
        let statement = StatementFixture::statement("(header-shape (let bindings) :bindings)");
        assert_eq!(
            statement.map(|statement| statement.shape),
            Some(Shape {
                header_length: 1,
                docstring: DocstringPlace::Absent,
                one_line: OneLineRule::AnyBody,
                one_per_line: false,
                body_lists: BodyLists::Forms,
                bindings: true
            })
        );
    }

    #[test]
    fn a_methods_marker_states_methods_alone() {
        let statement = StatementFixture::statement("(header-shape (impl trait type) :methods)");
        assert_eq!(
            statement.map(|statement| statement.shape),
            Some(Shape {
                header_length: 2,
                docstring: DocstringPlace::Absent,
                one_line: OneLineRule::AnyBody,
                one_per_line: false,
                body_lists: BodyLists::Methods,
                bindings: false
            })
        );
    }

    #[test]
    fn a_call_of_another_form_states_no_shape() {
        assert_eq!(StatementFixture::statement("(document (defn name))"), None);
    }

    #[test]
    fn a_shape_that_does_not_start_with_a_name_states_nothing() {
        assert_eq!(
            StatementFixture::statement("(header-shape (\"defn\" name))"),
            None
        );
    }
}
