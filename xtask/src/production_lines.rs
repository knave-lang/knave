use std::collections::BTreeSet;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{ImplItem, Item, TraitItem};

use crate::ratchet::Counts;
use crate::source_file::SourceFile;
use crate::test_code::TestCode;
use crate::test_modules::TestModuleFiles;

pub struct ProductionLineCensus;

impl ProductionLineCensus {
    const TEST_FILE_LINES: usize = 0;

    pub fn measure(sources: &[SourceFile]) -> Counts {
        let test_files = TestModuleFiles::find(sources);
        sources
            .iter()
            .map(|source| {
                let count = if test_files.contains(source) {
                    Self::TEST_FILE_LINES
                } else {
                    Self::count(source)
                };
                (source.relative().to_string(), count)
            })
            .collect()
    }

    fn count(source: &SourceFile) -> usize {
        let syntax = source.syntax();
        if TestCode::marks(&syntax.attrs) {
            return Self::TEST_FILE_LINES;
        }
        let mut test_lines = TestLines::default();
        test_lines.visit_file(syntax);
        source.line_count() - test_lines.excluded.len()
    }
}

#[derive(Default)]
struct TestLines {
    excluded: BTreeSet<usize>,
}

impl TestLines {
    fn exclude(&mut self, node: &impl Spanned) {
        let span = node.span();
        self.excluded.extend(span.start().line..=span.end().line);
    }
}

impl<'ast> Visit<'ast> for TestLines {
    fn visit_item(&mut self, item: &'ast Item) {
        if TestCode::marks_item(item) {
            self.exclude(item);
        } else {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if TestCode::marks_tokens(item) {
            self.exclude(item);
        } else {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        if TestCode::marks_tokens(item) {
            self.exclude(item);
        } else {
            visit::visit_trait_item(self, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ProductionLineCensus;
    use crate::ratchet::Counts;
    use crate::source_file::SourceFile;

    struct SourceText;

    impl SourceText {
        fn production_lines(text: &str) -> usize {
            Self::census(&[("src/a.rs", text)])["src/a.rs"]
        }

        fn census(files: &[(&str, &str)]) -> Counts {
            let sources: Vec<SourceFile> = files
                .iter()
                .map(|(path, text)| Self::source(path, text))
                .collect();
            ProductionLineCensus::measure(&sources)
        }

        fn source(path: &str, text: &str) -> SourceFile {
            SourceFile::parse(PathBuf::from(path), path.to_string(), text)
                .expect("the source parses")
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
    fn a_test_default_method_of_a_trait_does_not_count() {
        let text = "trait Shape {\n    #[cfg(test)]\n    fn sample() {}\n    fn area(&self);\n}\n";
        assert_eq!(SourceText::production_lines(text), 3);
    }

    #[test]
    fn an_inline_module_with_an_inner_test_condition_does_not_count() {
        let text = "mod tests {\n    #![cfg(test)]\n    fn helper() {}\n}\nfn area() {}\n";
        assert_eq!(SourceText::production_lines(text), 1);
    }

    #[test]
    fn an_item_under_all_conditions_with_test_does_not_count() {
        let text = "#[cfg(all(test, unix))]\nmod tests {\n    fn helper() {}\n}\nfn area() {}\n";
        assert_eq!(SourceText::production_lines(text), 1);
    }

    #[test]
    fn other_conditions_count() {
        let text = "#[cfg(unix)]\nfn on_unix() {}\n#[cfg(not(test))]\nfn outside_tests() {}\n#[allow(test)]\nfn unused() {}\n";
        assert_eq!(SourceText::production_lines(text), 6);
    }

    #[test]
    fn a_file_compiled_only_for_tests_counts_nothing() {
        let text = "#![cfg(test)]\n\nfn helper() {}\n";
        assert_eq!(SourceText::production_lines(text), 0);
    }

    #[test]
    fn a_file_declared_as_a_test_module_counts_nothing() {
        let census = SourceText::census(&[
            (
                "src/types/mod.rs",
                "#[cfg(test)]\nmod typed_source;\nmod kept;\n",
            ),
            (
                "src/types/typed_source.rs",
                "fn helper() {}\nfn other() {}\n",
            ),
            ("src/types/kept.rs", "fn area() {}\n"),
        ]);
        assert_eq!(census["src/types/mod.rs"], 1);
        assert_eq!(census["src/types/typed_source.rs"], 0);
        assert_eq!(census["src/types/kept.rs"], 1);
    }
}
