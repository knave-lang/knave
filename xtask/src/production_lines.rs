use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{ParseStream, Parser};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Ident, ImplItem, Item, TraitItem};

pub struct ProductionLines {
    test_lines: BTreeSet<usize>,
}

impl ProductionLines {
    const CONDITION: &str = "cfg";
    const TEST_CONDITION: &str = "test";

    pub fn count(text: &str) -> Result<usize, syn::Error> {
        let syntax = syn::parse_file(text)?;
        if Self::marks_test(&syntax.attrs) {
            return Ok(0);
        }
        let mut lines = Self {
            test_lines: BTreeSet::new(),
        };
        lines.visit_file(&syntax);
        Ok(text.lines().count() - lines.test_lines.len())
    }

    fn records_test_item(&mut self, node: &(impl ToTokens + Spanned)) -> bool {
        let is_test = Self::marks_test(&Self::outer_attributes(node));
        if is_test {
            let span = node.span();
            self.test_lines.extend(span.start().line..=span.end().line);
        }
        is_test
    }

    fn outer_attributes(node: &impl ToTokens) -> Vec<Attribute> {
        let attributes_then_anything = |input: ParseStream| {
            let attributes = Attribute::parse_outer(input)?;
            input.parse::<TokenStream>()?;
            Ok(attributes)
        };
        attributes_then_anything
            .parse2(node.to_token_stream())
            .unwrap_or_default()
    }

    fn marks_test(attributes: &[Attribute]) -> bool {
        attributes.iter().any(Self::is_test_condition)
    }

    fn is_test_condition(attribute: &Attribute) -> bool {
        attribute.path().is_ident(Self::CONDITION)
            && attribute
                .parse_args::<Ident>()
                .is_ok_and(|condition| condition == Self::TEST_CONDITION)
    }
}

impl<'ast> Visit<'ast> for ProductionLines {
    fn visit_item(&mut self, item: &'ast Item) {
        if !self.records_test_item(item) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if !self.records_test_item(item) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        if !self.records_test_item(item) {
            visit::visit_trait_item(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProductionLines;

    struct SourceText;

    impl SourceText {
        fn production_lines(text: &str) -> usize {
            ProductionLines::count(text).expect("the source parses")
        }
    }

    #[test]
    fn a_file_without_test_code_counts_every_physical_line() {
        let text = "fn area() -> u32 {\n    4\n}\n\nfn side() -> u32 {\n    2\n}\n";
        assert_eq!(SourceText::production_lines(text), 7);
    }

    #[test]
    fn a_test_module_and_its_attribute_do_not_count() {
        let text =
            "fn area() -> u32 {\n    4\n}\n\n#[cfg(test)]\nmod tests {\n    fn helper() {}\n}\n";
        assert_eq!(SourceText::production_lines(text), 4);
    }

    #[test]
    fn a_test_method_partway_through_an_impl_does_not_count() {
        let text = "\
struct Square;

impl Square {
    fn side(&self) -> u32 {
        2
    }

    /// Builds a square for the tests.
    #[cfg(test)]
    fn sample() -> Self {
        Square
    }

    fn area(&self) -> u32 {
        4
    }
}
";
        assert_eq!(SourceText::production_lines(text), 12);
    }

    #[test]
    fn a_test_item_partway_through_a_file_does_not_count() {
        let text = "\
#[cfg(test)]
pub struct Fixture;

#[cfg(test)]
impl Fixture {
    fn new() -> Self {
        Fixture
    }
}

fn area() -> u32 {
    4
}
";
        assert_eq!(SourceText::production_lines(text), 5);
    }

    #[test]
    fn a_test_item_inside_a_production_module_does_not_count() {
        let text = "mod shapes {\n    #[cfg(test)]\n    fn sample() {}\n\n    fn area() {}\n}\n";
        assert_eq!(SourceText::production_lines(text), 4);
    }

    #[test]
    fn a_test_default_method_of_a_trait_does_not_count() {
        let text = "trait Shape {\n    #[cfg(test)]\n    fn sample() {}\n    fn area(&self);\n}\n";
        assert_eq!(SourceText::production_lines(text), 3);
    }

    #[test]
    fn a_test_item_inside_a_production_method_does_not_count() {
        let text = "impl Square {\n    fn side() {\n        #[cfg(test)]\n        fn helper() {}\n    }\n}\n";
        assert_eq!(SourceText::production_lines(text), 4);
    }

    #[test]
    fn a_test_item_inside_a_default_trait_method_does_not_count() {
        let text = "trait Shape {\n    fn area() {\n        #[cfg(test)]\n        fn helper() {}\n    }\n}\n";
        assert_eq!(SourceText::production_lines(text), 4);
    }

    #[test]
    fn a_test_argument_of_another_attribute_counts() {
        let text = "#[allow(test)]\nfn area() {}\n";
        assert_eq!(SourceText::production_lines(text), 2);
    }

    #[test]
    fn other_conditions_count() {
        let text = "#[cfg(unix)]\nfn on_unix() {}\n#[cfg(not(test))]\nfn outside_tests() {}\n#[allow(dead_code)]\nfn unused() {}\n";
        assert_eq!(SourceText::production_lines(text), 6);
    }

    #[test]
    fn a_file_compiled_only_for_tests_counts_nothing() {
        let text = "#![cfg(test)]\n\nfn helper() {}\n";
        assert_eq!(SourceText::production_lines(text), 0);
    }

    #[test]
    fn unparsable_source_is_an_error() {
        assert!(ProductionLines::count("fn {").is_err());
    }
}
