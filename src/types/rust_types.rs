use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::template_spelling::TemplateSpelling;
use super::ty::Type;
use crate::core::{BindingId, ParsedTemplate, PrintFormat};

pub struct RustType {
    pub template: Rc<ParsedTemplate>,
    pub clauses: Vec<(Type, Rc<ParsedTemplate>)>,
    pub params: Vec<Rc<str>>,
    pub unsized_parameters: Vec<usize>,
    pub format: PrintFormat,
}

impl RustType {
    pub const SLICE_TEMPLATE: &'static str = "[{0}]";
    pub const VECTOR_TEMPLATE: &'static str = "Vec<{0}>";
    pub const BOX_TEMPLATE: &'static str = "Box<{0}>";
    pub const ITERATOR_TEMPLATE: &'static str = "impl Iterator<Item = {0}>";
    const SORTED_MAP_TEMPLATE: &'static str = "std::collections::BTreeMap<{0}, {1}>";
    const HASH_MAP_TEMPLATE: &'static str = "std::collections::HashMap<{0}, {1}>";
    const MAP_TEMPLATES: [&'static str; 2] = [Self::SORTED_MAP_TEMPLATE, Self::HASH_MAP_TEMPLATE];
    const OPAQUE_PREFIX: &'static str = "impl ";

    pub fn spells_a_map(template: &ParsedTemplate) -> bool {
        Self::MAP_TEMPLATES
            .iter()
            .any(|map| TemplateSpelling::of(template).is(map))
    }

    pub fn template_for(&self, argument: Option<&Type>) -> &Rc<ParsedTemplate> {
        argument
            .and_then(|argument| {
                self.clauses
                    .iter()
                    .find(|(target, _)| target == argument)
                    .map(|(_, template)| template)
            })
            .unwrap_or(&self.template)
    }

    pub fn is_opaque(&self) -> bool {
        self.template.text().starts_with(Self::OPAQUE_PREFIX)
    }

    pub fn has_template(&self, template: &str) -> bool {
        TemplateSpelling::of(&self.template).is(template)
    }

    pub fn leading_name(&self) -> &str {
        let text = self.template.text();
        let named = text.strip_prefix(Self::OPAQUE_PREFIX).unwrap_or(text);
        let end = named
            .find(|character: char| !character.is_alphanumeric() && character != '_')
            .unwrap_or(named.len());
        &named[..end]
    }
}

#[derive(Default)]
pub struct RustTypeTable {
    types: BTreeMap<BindingId, RustType>,
    iterators: BTreeSet<BindingId>,
}

impl RustTypeTable {
    pub fn add(&mut self, binding: BindingId, rust_type: RustType) {
        if rust_type.has_template(RustType::ITERATOR_TEMPLATE) {
            self.iterators.insert(binding);
        }
        self.types.insert(binding, rust_type);
    }

    pub fn get(&self, binding: BindingId) -> Option<&RustType> {
        self.types.get(&binding)
    }

    pub fn is_opaque(&self, binding: BindingId) -> bool {
        self.get(binding).is_some_and(RustType::is_opaque)
    }

    pub fn is_slice(&self, binding: BindingId) -> bool {
        self.has_template(binding, RustType::SLICE_TEMPLATE)
    }

    pub fn is_vector(&self, binding: BindingId) -> bool {
        self.has_template(binding, RustType::VECTOR_TEMPLATE)
    }

    pub fn is_box(&self, binding: BindingId) -> bool {
        self.has_template(binding, RustType::BOX_TEMPLATE)
    }

    pub fn parameter_may_be_unsized(&self, binding: BindingId, position: usize) -> bool {
        self.get(binding)
            .is_some_and(|rust_type| rust_type.unsized_parameters.contains(&position))
    }

    pub fn is_sorted_map(&self, binding: BindingId) -> bool {
        self.has_template(binding, RustType::SORTED_MAP_TEMPLATE)
    }

    pub fn is_hash_map(&self, binding: BindingId) -> bool {
        self.has_template(binding, RustType::HASH_MAP_TEMPLATE)
    }

    pub fn is_iterator(&self, binding: BindingId) -> bool {
        self.iterators.contains(&binding)
    }

    pub fn option_binding(&self) -> Option<BindingId> {
        self.binding_with_template("Option<{0}>")
    }

    pub fn iterator_binding(&self) -> Option<BindingId> {
        self.iterators.first().copied()
    }

    pub fn pair_binding(&self) -> Option<BindingId> {
        self.binding_with_template("({0}, {1})")
    }

    pub fn box_binding(&self) -> Option<BindingId> {
        self.binding_with_template(RustType::BOX_TEMPLATE)
    }

    pub fn binding_with_template(&self, template: &str) -> Option<BindingId> {
        self.types
            .iter()
            .find(|(_, rust_type)| rust_type.has_template(template))
            .map(|(binding, _)| *binding)
    }

    fn has_template(&self, binding: BindingId, template: &str) -> bool {
        self.get(binding)
            .is_some_and(|rust_type| rust_type.has_template(template))
    }
}

#[cfg(test)]
mod tests {
    use super::RustType;
    use crate::core::ParsedTemplate;

    struct TemplateFixture;

    impl TemplateFixture {
        fn spells_a_map(text: &str) -> bool {
            RustType::spells_a_map(&ParsedTemplate::parse(text).expect("a template"))
        }
    }

    #[test]
    fn a_sorted_or_hash_map_template_spells_a_map() {
        assert!(TemplateFixture::spells_a_map(
            "std::collections::BTreeMap<{0}, {1}>"
        ));
        assert!(TemplateFixture::spells_a_map(
            "std::collections::HashMap<{0},{1}>"
        ));
    }

    #[test]
    fn another_collection_template_spells_no_map() {
        assert!(!TemplateFixture::spells_a_map("Vec<{0}>"));
    }
}
