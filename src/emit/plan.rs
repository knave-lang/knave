use std::collections::BTreeSet;
use std::rc::Rc;

use knave_expand::mangle::NameMangler;
use rustc_hash::FxHashMap;

use super::constant_definitions::ConstantDefinitions;
use super::entry_point::EntryPoint;
use super::item_visibilities::ItemVisibilities;
use super::problems::EmitProblems;
use super::template::Template;
use super::template::judged_templates::JudgedTemplates;
use super::trait_use_search::TraitUseSearch;
use super::visibility::Visibility;
use crate::core::{BindingId, ModuleId};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::source::Span;
use crate::types::{DefinitionTable, ProgramReach, TypedDefinition, TypedExprKind, TypedProgram};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Function,
    Const,
    Static,
    Struct,
    Enum,
}

pub struct ItemInfo {
    pub module: ModuleId,
    pub name: String,
    pub kind: ItemKind,
    pub visibility: Visibility,
}

pub struct EmitPlan<'p> {
    program: &'p TypedProgram,
    entry: EntryPoint,
    definitions: DefinitionTable<'p>,
    reach: &'p ProgramReach,
    visibilities: ItemVisibilities,
    items: FxHashMap<BindingId, ItemInfo>,
    trait_uses: FxHashMap<ModuleId, BTreeSet<BindingId>>,
    templates: JudgedTemplates,
}

impl<'p> EmitPlan<'p> {
    const ENTRY_RUST_NAME: &'static str = "main";
    const MODULE_SEPARATOR: char = '/';
    const FIRST_RENAME_SUFFIX: usize = 2;

    pub fn build(program: &'p TypedProgram, entry_span: Span) -> Result<Self, Diagnostics> {
        let reach = &program.ownership.reach;
        let main = reach
            .main
            .ok_or_else(|| Diagnostics::from(EmitProblems::missing_main(entry_span)))?;
        let mut plan = Self {
            program,
            entry: EntryPoint::new(main),
            definitions: DefinitionTable::of(program),
            reach,
            visibilities: ItemVisibilities::new(
                program
                    .modules
                    .iter()
                    .flat_map(|module| module.exports.iter().copied())
                    .collect(),
                reach.used_from_other_modules.clone(),
            ),
            items: FxHashMap::default(),
            trait_uses: FxHashMap::default(),
            templates: JudgedTemplates::default(),
        };
        plan.entry.name_body(plan.choose_main_body_name());
        plan.assign_items()?;
        plan.collect_trait_uses();
        Ok(plan)
    }

    pub fn reached(&self) -> &[BindingId] {
        &self.reach.reached
    }

    pub fn item(&self, binding: BindingId) -> Option<&ItemInfo> {
        self.items.get(&binding)
    }

    pub fn entry(&self) -> &EntryPoint {
        &self.entry
    }

    pub fn template(&self, text: &Rc<str>) -> Template {
        self.templates.template(text)
    }

    fn choose_main_body_name(&self) -> Option<String> {
        self.definition(self.entry.function())?
            .result_error(self.program.declarations.types().enums())?;
        let taken: BTreeSet<String> = self
            .reach
            .reached
            .iter()
            .filter(|binding| **binding != self.entry.function())
            .filter(|binding| !self.program.synthetic_bindings.contains(*binding))
            .map(|binding| self.item_info(*binding))
            .filter(|info| info.module == self.program.entry)
            .map(|info| info.name)
            .collect();
        let mut name = "main_body".to_string();
        while taken.contains(&name) {
            name.push('_');
        }
        Some(name)
    }

    pub fn definition(&self, binding: BindingId) -> Option<&'p TypedDefinition> {
        self.definitions.definition(binding)
    }

    pub fn items_of(&self, module: ModuleId) -> Vec<BindingId> {
        let mut bindings: Vec<BindingId> = self
            .items
            .iter()
            .filter(|(_, info)| info.module == module)
            .map(|(binding, _)| *binding)
            .collect();
        bindings.sort();
        bindings
    }

    pub fn path_to(&self, binding: BindingId, from: ModuleId) -> String {
        let info = &self.items[&binding];
        if info.module == from {
            return info.name.clone();
        }
        let mut segments = vec!["crate".to_string()];
        segments.extend(self.module_segments(info.module));
        segments.push(info.name.clone());
        segments.join("::")
    }

    pub fn trait_path(&self, binding: BindingId, from: ModuleId) -> String {
        let traits = &self.program.declarations.traits();
        if let Some(path) = traits.rust_paths().written(binding) {
            return path.to_string();
        }
        let definition = traits
            .definitions()
            .iter()
            .find(|trait_definition| trait_definition.binding == binding)
            .expect("a trait method names a trait definition");
        if definition.module == from {
            return definition.rust_name.to_string();
        }
        let mut segments = vec!["crate".to_string()];
        segments.extend(self.module_segments(definition.module));
        segments.push(definition.rust_name.to_string());
        segments.join("::")
    }

    pub fn trait_module(&self, binding: BindingId) -> Option<ModuleId> {
        let traits = &self.program.declarations.traits();
        traits
            .definitions()
            .iter()
            .find(|trait_definition| trait_definition.binding == binding)
            .map(|trait_definition| trait_definition.module)
    }

    pub fn trait_uses(&self, module: ModuleId) -> Option<&BTreeSet<BindingId>> {
        self.trait_uses.get(&module)
    }

    pub fn uses_trait(&self, module: ModuleId, binding: BindingId) -> bool {
        self.trait_uses
            .get(&module)
            .is_some_and(|uses| uses.contains(&binding))
    }

    fn collect_trait_uses(&mut self) {
        let modules: Vec<ModuleId> = self
            .program
            .modules
            .iter()
            .map(|module| module.id)
            .collect();
        for module in modules {
            let mut uses = BTreeSet::new();
            self.collect_module_trait_uses(module, &mut uses);
            self.collect_hidden_trait_uses(module, &mut uses);
            if !uses.is_empty() {
                self.trait_uses.insert(module, uses);
            }
        }
    }

    fn collect_module_trait_uses(&self, module: ModuleId, uses: &mut BTreeSet<BindingId>) {
        for binding in self.items_of(module) {
            if let Some(definition) = self.definition(binding) {
                TraitUseSearch::new(self.program).collect(&definition.value, uses);
            }
        }
    }

    fn collect_hidden_trait_uses(&self, module: ModuleId, uses: &mut BTreeSet<BindingId>) {
        let traits = &self.program.declarations.traits();
        let impl_methods = traits
            .impls()
            .iter()
            .filter(|implementation| implementation.module == module)
            .flat_map(|implementation| implementation.methods.iter())
            .map(|method| method.binding);
        let defaults = traits
            .definitions()
            .iter()
            .filter(|trait_definition| trait_definition.module == module)
            .flat_map(|trait_definition| trait_definition.methods.iter())
            .filter_map(|method| method.default);
        for binding in impl_methods.chain(defaults) {
            if let Some(definition) = self.definition(binding) {
                TraitUseSearch::new(self.program).collect(&definition.value, uses);
            }
        }
    }

    pub fn module_segments(&self, module: ModuleId) -> Vec<String> {
        if module == self.program.entry {
            return Vec::new();
        }
        let path = &self
            .program
            .modules
            .iter()
            .find(|candidate| candidate.id == module)
            .expect("module exists")
            .path;
        path.as_str()
            .split(Self::MODULE_SEPARATOR)
            .map(NameMangler::value)
            .collect()
    }

    pub fn rust_names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.items.iter().map(|(binding, info)| {
            (
                info.name.as_str(),
                self.program.bindings.get(*binding).name(),
            )
        })
    }

    fn assign_items(&mut self) -> Result<(), Diagnostics> {
        let mut taken: FxHashMap<(ModuleId, String), BindingId> = FxHashMap::default();
        let mut diagnostics = Diagnostics::default();
        for binding in self.entry_and_written_items_first() {
            let info = self.item_info(binding);
            let key = (info.module, info.name.clone());
            match taken
                .get(&key)
                .copied()
                .or_else(|| self.shadowed_rust_type(&info))
            {
                Some(other) if self.yields_name(binding, other) => {
                    let renamed = Self::next_free_name(info, &taken);
                    self.claim(&mut taken, binding, renamed);
                }
                Some(other) => diagnostics.push(self.collision(&info.name, other, binding)),
                None => self.claim(&mut taken, binding, info),
            }
        }
        diagnostics.into_result(())
    }

    fn yields_name(&self, binding: BindingId, holder: BindingId) -> bool {
        holder == self.entry.function() || self.program.bindings.origin(binding).is_introduced()
    }

    fn entry_and_written_items_first(&self) -> Vec<BindingId> {
        let (introduced, written): (Vec<BindingId>, Vec<BindingId>) = self
            .reach
            .reached
            .iter()
            .copied()
            .filter(|binding| *binding != self.entry.function())
            .filter(|binding| !self.program.synthetic_bindings.contains(binding))
            .partition(|binding| self.program.bindings.origin(*binding).is_introduced());
        std::iter::once(self.entry.function())
            .chain(written)
            .chain(introduced)
            .collect()
    }

    fn claim(
        &mut self,
        taken: &mut FxHashMap<(ModuleId, String), BindingId>,
        binding: BindingId,
        info: ItemInfo,
    ) {
        taken.insert((info.module, info.name.clone()), binding);
        self.items.insert(binding, info);
    }

    fn next_free_name(
        info: ItemInfo,
        taken: &FxHashMap<(ModuleId, String), BindingId>,
    ) -> ItemInfo {
        let separator = Self::rename_separator(&info);
        let name = (Self::FIRST_RENAME_SUFFIX..)
            .map(|suffix| format!("{}{separator}{suffix}", info.name))
            .find(|candidate| !taken.contains_key(&(info.module, candidate.clone())))
            .expect("an unbounded range holds a free name");
        ItemInfo { name, ..info }
    }

    fn rename_separator(info: &ItemInfo) -> &'static str {
        if matches!(info.kind, ItemKind::Struct | ItemKind::Enum) {
            ""
        } else {
            "_"
        }
    }

    fn shadowed_rust_type(&self, info: &ItemInfo) -> Option<BindingId> {
        let is_type = matches!(info.kind, ItemKind::Struct | ItemKind::Enum);
        let rust_types = self.program.declarations.types().rust_types();
        self.reach.used_rust_types.iter().copied().find(|binding| {
            is_type
                && rust_types
                    .get(*binding)
                    .is_some_and(|rust_type| rust_type.leading_name() == info.name)
        })
    }

    fn item_info(&self, binding: BindingId) -> ItemInfo {
        let name = self.program.bindings.get(binding).name();
        let (module, kind) = match self.definitions.entry(binding) {
            Some((definition, module)) => (module, self.definition_kind(definition)),
            None => (
                self.reach.type_modules[&binding],
                self.named_type_kind(binding),
            ),
        };
        let rust_name = match (
            kind,
            binding == self.entry.function(),
            self.entry.body_name(),
        ) {
            (ItemKind::Function, true, Some(body)) => body.to_string(),
            (ItemKind::Function, true, None) => Self::ENTRY_RUST_NAME.to_string(),
            (ItemKind::Function, ..) => NameMangler::value(name),
            (ItemKind::Const | ItemKind::Static, ..) => NameMangler::constant(name),
            (ItemKind::Struct | ItemKind::Enum, ..) => NameMangler::type_name(name),
        };
        ItemInfo {
            module,
            name: rust_name,
            kind,
            visibility: self.visibilities.of(binding),
        }
    }

    fn named_type_kind(&self, binding: BindingId) -> ItemKind {
        match self.program.declarations.types().enums().get(binding) {
            Some(_) => ItemKind::Enum,
            None => ItemKind::Struct,
        }
    }

    fn definition_kind(&self, definition: &TypedDefinition) -> ItemKind {
        match &definition.value.kind {
            TypedExprKind::Fn(_) => ItemKind::Function,
            _ if ConstantDefinitions::new(self.program, &self.definitions)
                .is_const_evaluable(&definition.value) =>
            {
                ItemKind::Const
            }
            _ => ItemKind::Static,
        }
    }

    fn collision(&self, rust_name: &str, first: BindingId, second: BindingId) -> Diagnostic {
        let first_info = self.program.bindings.get(first);
        let second_info = self.program.bindings.get(second);
        let span = second_info
            .span()
            .or(first_info.span())
            .expect("user definitions have spans");
        EmitProblems::name_collision(
            rust_name,
            first_info.name(),
            second_info.name(),
            span,
            first_info.span(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::emit::RustEmitter;
    use crate::source::{SourceId, Span};
    use crate::types::{ReachSearch, TypedSource};

    struct EntryFixture {
        source: TypedSource,
    }

    impl EntryFixture {
        const ENTRY_BESIDE_A_WRITTEN_MAIN: &str = "
            (defn main () (println \"written\"))
            (defn start () (main))";

        fn entering_at(name: &str) -> Self {
            let mut source = TypedSource::expand(Self::ENTRY_BESIDE_A_WRITTEN_MAIN);
            source.enter_at(name);
            Self { source }
        }

        fn emitted(&self) -> String {
            let mut program = self.source.typed();
            program.ownership.reach = ReachSearch::of(&program);
            RustEmitter::emit(&program, Span::new(SourceId::new(0), 0..0))
                .unwrap_or_else(|_| panic!("the fixture emits"))
                .code
        }
    }

    #[test]
    fn the_entry_takes_the_rust_name_main_from_a_written_main() {
        let emitted = EntryFixture::entering_at("start").emitted();
        assert!(emitted.contains("fn main() {"), "{emitted}");
        assert!(emitted.contains("fn main_2() {"), "{emitted}");
        assert!(!emitted.contains("fn start"), "{emitted}");
    }

    #[test]
    fn a_written_main_that_is_the_entry_keeps_its_name() {
        let emitted = EntryFixture::entering_at("main").emitted();
        assert!(emitted.contains("fn main() {"), "{emitted}");
        assert!(!emitted.contains("main_2"), "{emitted}");
    }
}
