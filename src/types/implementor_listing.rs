use rustc_hash::FxHashSet;

use super::explanation_sources::ExplanationSources;
use super::problems::{ImplementorGroup, ImplementorList};
use super::ty::Type;
use super::{Implementor, RustTrait};
use crate::core::BindingId;

pub(super) struct ImplementorListing<'s, 'a> {
    sources: &'s ExplanationSources<'a>,
}

impl<'s, 'a> ImplementorListing<'s, 'a> {
    pub(super) fn new(sources: &'s ExplanationSources<'a>) -> Self {
        Self { sources }
    }

    pub(super) fn list(&self, trait_binding: BindingId) -> ImplementorList {
        self.sources
            .declarations
            .traits()
            .table
            .get(trait_binding)
            .map(|rust_trait| self.group_implementors(rust_trait))
            .unwrap_or_default()
    }

    fn group_implementors(&self, rust_trait: &RustTrait) -> ImplementorList {
        let display = &self.sources.display;
        let implementors = rust_trait.rows().reached().implementors();
        let mut grouped: Vec<(BindingId, Vec<(usize, String)>)> = Vec::new();
        for (position, known) in implementors.iter().enumerate() {
            let Some((binding, argument)) = self.grouped_implementor(known) else {
                continue;
            };
            let shown = display.show(argument);
            match grouped.iter_mut().find(|(group, _)| *group == binding) {
                Some((_, entries)) => entries.push((position, shown)),
                None => grouped.push((binding, vec![(position, shown)])),
            }
        }
        let groups: Vec<(BindingId, Vec<(usize, String)>)> = grouped
            .into_iter()
            .filter(|(_, entries)| entries.len() > 1)
            .collect();
        let grouped_positions: FxHashSet<usize> = groups
            .iter()
            .flat_map(|(_, entries)| entries.iter().map(|(position, _)| *position))
            .collect();
        let entries = implementors
            .iter()
            .enumerate()
            .filter(|(position, _)| !grouped_positions.contains(position))
            .map(|(_, known)| self.show_implementor(known))
            .collect();
        let groups = groups
            .into_iter()
            .map(|(binding, entries)| ImplementorGroup {
                constructor: self.sources.bindings.get(binding).name().to_string(),
                parameter: self.group_parameter(binding),
                arguments: entries.into_iter().map(|(_, shown)| shown).collect(),
            })
            .collect();
        ImplementorList { entries, groups }
    }

    fn group_parameter(&self, binding: BindingId) -> String {
        let declarations = self.sources.declarations;
        let projected = declarations
            .traits()
            .table
            .projections()
            .associated(binding)
            .map(|associated| associated.parameter.to_string());
        projected
            .or_else(|| {
                declarations
                    .types()
                    .rust_types
                    .get(binding)
                    .and_then(|rust_type| rust_type.params.first())
                    .map(ToString::to_string)
            })
            .expect("a grouped implementor has a type parameter")
    }

    fn grouped_implementor<'k>(&self, known: &'k Implementor) -> Option<(BindingId, &'k Type)> {
        if known.argument.is_some() {
            return None;
        }
        let Type::Named(binding, arguments) = &known.scheme.body else {
            return None;
        };
        let [argument] = arguments.as_slice() else {
            return None;
        };
        let declarations = self.sources.declarations;
        let single_parameter = declarations
            .traits()
            .table
            .projections()
            .is_projection(*binding)
            || declarations
                .types()
                .rust_types
                .get(*binding)
                .is_some_and(|rust_type| rust_type.params.len() == 1);
        (single_parameter && !argument.is_unknown()).then_some((*binding, argument))
    }

    fn show_implementor(&self, implementor: &Implementor) -> String {
        let display = &self.sources.display;
        let body = display.scheme(&implementor.scheme);
        match &implementor.argument {
            Some(argument) => format!("(with {body} {})", display.show(argument)),
            None => body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImplementorListing;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::problems::ImplementorList;

    const SOURCE: &str = "
        (trait shown (show (fn ((ref self)) usize)))
        (impl shown (vec i64) (show (items) (len items)))
        (impl shown (vec string) (show (items) (len items)))
        (impl shown (option i64) (show (item) 1))
        (impl shown i64 (show (item) 1))
        (rust-trait matching \"std::cmp::PartialEq\" (with (vec i64) i64) (with (vec u8) u8))
        (rust-trait listed \"std::fmt::Debug\" (where (a) (option a)) (option i64) (option u8))
        (defn main () (do))";

    struct ListingFixture {
        declarations: DeclarationsFixture,
    }

    impl ListingFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn listed(&self, trait_name: &str) -> ImplementorList {
            let sources = self.declarations.explanation_sources();
            ImplementorListing::new(&sources).list(self.declarations.trait_named(trait_name))
        }
    }

    #[test]
    fn implementors_of_one_constructor_are_grouped() {
        let listed = ListingFixture::new().listed("shown");
        let groups: Vec<(&str, &str, Vec<&str>)> = (listed.groups.iter())
            .map(|group| {
                let arguments = group.arguments.iter().map(String::as_str).collect();
                (
                    group.constructor.as_str(),
                    group.parameter.as_str(),
                    arguments,
                )
            })
            .collect();
        assert_eq!(groups, [("vec", "item", vec!["int", "string"])]);
    }

    #[test]
    fn a_lone_implementor_of_a_constructor_stays_an_entry() {
        assert_eq!(
            ListingFixture::new().listed("shown").entries,
            ["(option int)", "int"]
        );
    }

    #[test]
    fn implementors_with_a_second_type_are_never_grouped() {
        let listed = ListingFixture::new().listed("matching");
        assert!(listed.groups.is_empty());
        assert_eq!(
            listed.entries,
            ["(with (vec int) int)", "(with (vec u8) u8)"]
        );
    }

    #[test]
    fn a_generic_implementor_stays_an_entry() {
        let listed = ListingFixture::new().listed("listed");
        assert_eq!(listed.entries, ["(where (a) (option a))"]);
        assert_eq!(listed.groups[0].arguments, ["int", "u8"]);
    }
}
