use std::collections::BTreeSet;

use syn::Item;

use super::use_bindings::UseBindings;

pub struct ModuleNames {
    defined: BTreeSet<String>,
    bindings: UseBindings,
}

impl ModuleNames {
    pub fn of(syntax: &syn::File) -> Self {
        Self {
            defined: syntax.items.iter().filter_map(Self::name_of).collect(),
            bindings: UseBindings::of(syntax),
        }
    }

    pub fn defines(&self, name: &str) -> bool {
        self.defined.contains(name)
    }

    pub fn offers(&self, name: &str) -> bool {
        self.defines(name) || self.bindings.path_of(name).is_some()
    }

    pub fn bindings(&self) -> &UseBindings {
        &self.bindings
    }

    fn name_of(item: &Item) -> Option<String> {
        let ident = match item {
            Item::Const(item) => &item.ident,
            Item::Enum(item) => &item.ident,
            Item::Fn(item) => &item.sig.ident,
            Item::Macro(item) => item.ident.as_ref()?,
            Item::Mod(item) => &item.ident,
            Item::Static(item) => &item.ident,
            Item::Struct(item) => &item.ident,
            Item::Trait(item) => &item.ident,
            Item::TraitAlias(item) => &item.ident,
            Item::Type(item) => &item.ident,
            Item::Union(item) => &item.ident,
            _ => return None,
        };
        Some(ident.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleNames;

    struct Names;

    impl Names {
        fn of(text: &str) -> ModuleNames {
            ModuleNames::of(&syn::parse_file(text).expect("the module parses"))
        }
    }

    #[test]
    fn every_named_item_is_defined() {
        let names = Names::of(
            "const A: u8 = 0; enum B {} fn c() {} macro_rules! d { () => {} } mod e {} static F: u8 = 0; struct G; trait H {} type I = u8; union J { k: u8 }",
        );
        for name in ["A", "B", "c", "d", "e", "F", "G", "H", "I", "J"] {
            assert!(names.defines(name), "{name}");
        }
    }

    #[test]
    fn a_use_offers_a_name_without_defining_it() {
        let names = Names::of("pub use scheme::Scheme; impl Scheme {}");
        assert!(!names.defines("Scheme"));
        assert!(names.offers("Scheme"));
        assert!(!names.offers("Other"));
    }
}
