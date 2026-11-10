use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{Attribute, Block, Item, ItemImpl, ItemMod, ItemTrait, Meta, Stmt, Token};

pub struct TestCode;

impl TestCode {
    const CONDITION: &str = "cfg";
    const TEST: &str = "test";
    const ALL: &str = "all";

    pub fn marks(attributes: &[Attribute]) -> bool {
        attributes.iter().any(Self::is_test_condition)
    }

    pub fn marks_item(item: &Item) -> bool {
        match item {
            Item::Mod(module) => Self::marks(&module.attrs),
            other => Self::marks_tokens(other),
        }
    }

    pub fn strip(syntax: &mut syn::File) {
        TestItemRemover.visit_file_mut(syntax);
    }

    pub fn marks_tokens(node: &impl ToTokens) -> bool {
        Self::marks(&Self::outer_attributes(node))
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

    fn is_test_condition(attribute: &Attribute) -> bool {
        attribute.path().is_ident(Self::CONDITION)
            && attribute
                .parse_args::<Meta>()
                .is_ok_and(|condition| Self::requires_test(&condition))
    }

    fn requires_test(condition: &Meta) -> bool {
        match condition {
            Meta::Path(path) => path.is_ident(Self::TEST),
            Meta::List(list) if list.path.is_ident(Self::ALL) => list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .is_ok_and(|conditions| conditions.iter().any(Self::requires_test)),
            _ => false,
        }
    }
}

struct TestItemRemover;

impl VisitMut for TestItemRemover {
    fn visit_file_mut(&mut self, syntax: &mut syn::File) {
        syntax.items.retain(|item| !TestCode::marks_item(item));
        visit_mut::visit_file_mut(self, syntax);
    }

    fn visit_item_mod_mut(&mut self, module: &mut ItemMod) {
        if let Some((_, items)) = &mut module.content {
            items.retain(|item| !TestCode::marks_item(item));
        }
        visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, implementation: &mut ItemImpl) {
        implementation
            .items
            .retain(|item| !TestCode::marks_tokens(item));
        visit_mut::visit_item_impl_mut(self, implementation);
    }

    fn visit_item_trait_mut(&mut self, definition: &mut ItemTrait) {
        definition
            .items
            .retain(|item| !TestCode::marks_tokens(item));
        visit_mut::visit_item_trait_mut(self, definition);
    }

    fn visit_block_mut(&mut self, block: &mut Block) {
        block.stmts.retain(
            |statement| !matches!(statement, Stmt::Item(item) if TestCode::marks_item(item)),
        );
        visit_mut::visit_block_mut(self, block);
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::TestCode;

    struct Snippet;

    impl Snippet {
        fn is_test(text: &str) -> bool {
            TestCode::marks_item(&syn::parse_str(text).expect("the item parses"))
        }

        fn stripped(text: &str) -> String {
            let mut syntax = syn::parse_file(text).expect("the source parses");
            TestCode::strip(&mut syntax);
            syntax.to_token_stream().to_string()
        }
    }

    #[test]
    fn a_test_condition_marks_test_code() {
        assert!(Snippet::is_test("#[cfg(test)] fn helper() {}"));
    }

    #[test]
    fn a_test_condition_among_all_conditions_marks_test_code() {
        assert!(Snippet::is_test("#[cfg(all(unix, test))] fn helper() {}"));
        assert!(Snippet::is_test(
            "#[cfg(all(unix, all(test)))] fn helper() {}"
        ));
    }

    #[test]
    fn an_inner_test_condition_marks_an_inline_module() {
        assert!(Snippet::is_test(
            "mod tests { #![cfg(test)] fn helper() {} }"
        ));
    }

    #[test]
    fn other_conditions_mark_production_code() {
        assert!(!Snippet::is_test("#[cfg(unix)] fn on_unix() {}"));
        assert!(!Snippet::is_test("#[cfg(not(test))] fn outside() {}"));
        assert!(!Snippet::is_test("#[cfg(any(test, unix))] fn either() {}"));
        assert!(!Snippet::is_test(
            "#[cfg(all(unix, windows))] fn neither() {}"
        ));
        assert!(!Snippet::is_test("#[allow(test)] fn allowed() {}"));
        assert!(!Snippet::is_test("mod shapes { fn area() {} }"));
    }

    #[test]
    fn strips_test_items_from_files_modules_impls_traits_and_blocks() {
        let text = "\
#[cfg(test)] fn file_helper() {}
mod shapes { #[cfg(test)] fn module_helper() {} fn kept() {} }
impl Square { #[cfg(test)] fn sample() {} fn side() {} }
trait Shape { #[cfg(test)] fn sample() {} fn area(); }
fn body() { #[cfg(test)] fn block_helper() {} let kept = 1; }
mod inner { #![cfg(test)] fn helper() {} }
";
        assert_eq!(
            Snippet::stripped(text),
            "mod shapes { fn kept () { } } impl Square { fn side () { } } trait Shape { fn area () ; } fn body () { let kept = 1 ; }"
        );
    }

    #[test]
    fn keeps_items_under_other_conditions() {
        assert_eq!(
            Snippet::stripped("#[cfg(unix)] fn on_unix() {}"),
            "# [cfg (unix)] fn on_unix () { }"
        );
    }
}
