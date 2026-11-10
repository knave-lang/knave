use syn::visit::{self, Visit};
use syn::{FnArg, ImplItemFn, ItemFn, ItemImpl, ItemTrait, Signature, TraitItemFn, Type};

use crate::functions::FunctionCollector;
use crate::parsed_source::ParsedSource;
use crate::ratchet::Counts;

#[derive(Default)]
pub struct Signatures {
    pub arguments: Counts,
    pub bool_parameters: Counts,
    pub free_functions: Counts,
}

impl Signatures {
    const ENTRY_POINT: &str = "main";
    const TEST_ATTRIBUTE: &str = "test";
    const BOOL: &str = "bool";

    pub fn measure(sources: &[ParsedSource]) -> Self {
        let mut signatures = Self::default();
        for source in sources {
            SignatureVisitor {
                file: source.file(),
                owner: None,
                signatures: &mut signatures,
            }
            .visit_file(source.syntax());
        }
        signatures
    }

    fn record(&mut self, function: String, signature: &Signature) {
        let parameters: Vec<&Type> = signature
            .inputs
            .iter()
            .filter_map(|input| match input {
                FnArg::Typed(parameter) => Some(parameter.ty.as_ref()),
                FnArg::Receiver(_) => None,
            })
            .collect();
        let bools = parameters.iter().filter(|ty| Self::is_bool(ty)).count();
        Self::record_largest(&mut self.arguments, function.clone(), parameters.len());
        Self::record_largest(&mut self.bool_parameters, function, bools);
    }

    fn record_free(&mut self, function: String, definition: &ItemFn) {
        let is_test = definition
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident(Self::TEST_ATTRIBUTE));
        let count = self.free_functions.entry(function).or_default();
        if definition.sig.ident != Self::ENTRY_POINT && !is_test {
            *count += 1;
        }
    }

    fn record_largest(counts: &mut Counts, function: String, count: usize) {
        let recorded = counts.entry(function).or_default();
        *recorded = (*recorded).max(count);
    }

    fn is_bool(ty: &Type) -> bool {
        matches!(ty, Type::Path(path) if path.path.is_ident(Self::BOOL))
    }
}

struct SignatureVisitor<'a> {
    file: &'a str,
    owner: Option<String>,
    signatures: &'a mut Signatures,
}

impl SignatureVisitor<'_> {
    fn function(&self, signature: &Signature) -> String {
        match &self.owner {
            Some(owner) => format!("{}::{owner}::{}", self.file, signature.ident),
            None => format!("{}::{}", self.file, signature.ident),
        }
    }

    fn within_owner(&mut self, owner: Option<String>, visit: impl FnOnce(&mut Self)) {
        let outer = std::mem::replace(&mut self.owner, owner);
        visit(self);
        self.owner = outer;
    }
}

impl<'ast> Visit<'ast> for SignatureVisitor<'_> {
    fn visit_item_fn(&mut self, definition: &'ast ItemFn) {
        let function = self.function(&definition.sig);
        self.signatures.record(function.clone(), &definition.sig);
        self.signatures.record_free(function, definition);
        visit::visit_item_fn(self, definition);
    }

    fn visit_item_impl(&mut self, implementation: &'ast ItemImpl) {
        self.within_owner(FunctionCollector::type_name(implementation), |visitor| {
            visit::visit_item_impl(visitor, implementation);
        });
    }

    fn visit_item_trait(&mut self, definition: &'ast ItemTrait) {
        self.within_owner(Some(definition.ident.to_string()), |visitor| {
            visit::visit_item_trait(visitor, definition);
        });
    }

    fn visit_impl_item_fn(&mut self, method: &'ast ImplItemFn) {
        self.signatures
            .record(self.function(&method.sig), &method.sig);
        visit::visit_impl_item_fn(self, method);
    }

    fn visit_trait_item_fn(&mut self, method: &'ast TraitItemFn) {
        self.signatures
            .record(self.function(&method.sig), &method.sig);
        visit::visit_trait_item_fn(self, method);
    }
}

#[cfg(test)]
mod tests {
    use super::Signatures;
    use crate::parsed_source::ParsedSource;

    struct Source;

    impl Source {
        fn signatures(text: &str) -> Signatures {
            Signatures::measure(&[ParsedSource::sample("src/a.rs", text)])
        }
    }

    #[test]
    fn counts_the_arguments_of_functions_methods_and_trait_methods_without_self() {
        let signatures = Source::signatures(
            "fn free(a: u8, b: u8) {} impl Square { fn method(&self, a: u8, b: u8, c: u8, d: u8) {} } trait Shape { fn area(&self, scale: u8); }",
        );
        assert_eq!(signatures.arguments.get("src/a.rs::free"), Some(&2));
        assert_eq!(
            signatures.arguments.get("src/a.rs::Square::method"),
            Some(&4)
        );
        assert_eq!(signatures.arguments.get("src/a.rs::Shape::area"), Some(&1));
    }

    #[test]
    fn counts_bool_parameters() {
        let signatures = Source::signatures(
            "impl Square { fn draw(&self, filled: bool, outlined: bool, width: u8, shown: &bool) {} }",
        );
        assert_eq!(
            signatures.bool_parameters.get("src/a.rs::Square::draw"),
            Some(&2)
        );
    }

    #[test]
    fn counts_free_functions_but_not_main_tests_or_methods() {
        let signatures = Source::signatures(
            "fn main() {} fn helper() {} #[test] fn checks() {} impl Square { fn side() { fn nested() {} } }",
        );
        assert_eq!(signatures.free_functions.get("src/a.rs::helper"), Some(&1));
        assert_eq!(
            signatures.free_functions.get("src/a.rs::Square::nested"),
            Some(&1)
        );
        assert_eq!(signatures.free_functions.get("src/a.rs::main"), Some(&0));
        assert_eq!(signatures.free_functions.get("src/a.rs::checks"), Some(&0));
        assert_eq!(
            signatures.free_functions.get("src/a.rs::Square::side"),
            None
        );
    }
}
