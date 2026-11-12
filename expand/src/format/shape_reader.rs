use std::path::Path;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::shapes::{ShapeCatalog, ShapeStatement};
use crate::core::{DefinitionForm, KernelForm, ModulePath};
use crate::modules::{LocatedModule, ModuleSource, ModuleSources};
use crate::reader::{Form, Reader};
use crate::source::SourceMap;

pub struct ModuleLookup {
    directories: Vec<ModuleSources>,
    shape_modules: Vec<ModulePath>,
}

#[derive(Default)]
pub struct ShapeReader {
    modules: FxHashMap<String, Rc<ModuleShapes>>,
}

#[derive(Default)]
struct ModuleShapes {
    statements: Vec<ShapeStatement>,
    uses: Vec<ModulePath>,
}

struct CatalogWalk<'r> {
    reader: &'r mut ShapeReader,
    lookup: &'r ModuleLookup,
    visited: FxHashSet<String>,
    catalog: ShapeCatalog,
}

impl ModuleLookup {
    pub fn beside(directory: &Path) -> Self {
        Self {
            directories: directory
                .ancestors()
                .map(|ancestor| ModuleSources::new(ancestor.to_path_buf(), ancestor.to_path_buf()))
                .collect(),
            shape_modules: Vec::new(),
        }
    }

    pub fn within_project(project_sources: ModuleSources) -> Self {
        Self {
            directories: vec![project_sources],
            shape_modules: Vec::new(),
        }
    }

    pub fn beside_project_file(directory: &Path) -> Self {
        let mut lookup = Self::beside(directory);
        lookup
            .shape_modules
            .push(ModulePath::new(ModuleSources::PROJECT_SHAPES));
        lookup
    }

    fn find(&self, owner: Option<&str>, written: &ModulePath) -> Option<LocatedModule> {
        self.directories.iter().find_map(|sources| {
            let located = sources.locate(&sources.resolve(owner, written))?;
            matches!(located.source, ModuleSource::Text { .. }).then_some(located)
        })
    }
}

impl ShapeReader {
    pub fn catalog(&mut self, forms: &[Form], lookup: &ModuleLookup) -> ShapeCatalog {
        let file = ModuleShapes::of(forms);
        self.walk(lookup, |walk| walk.add_shapes(None, &file))
    }

    pub fn catalog_of_module(
        &mut self,
        module: &ModulePath,
        lookup: &ModuleLookup,
    ) -> ShapeCatalog {
        self.walk(lookup, |walk| walk.add_module(None, module))
    }

    fn walk(
        &mut self,
        lookup: &ModuleLookup,
        add_own_shapes: impl FnOnce(&mut CatalogWalk<'_>),
    ) -> ShapeCatalog {
        let mut walk = CatalogWalk {
            reader: self,
            lookup,
            visited: FxHashSet::default(),
            catalog: ShapeCatalog::default(),
        };
        walk.add_module(None, &ModulePath::new(ModuleSources::PRELUDE));
        lookup
            .shape_modules
            .iter()
            .for_each(|module| walk.add_module(None, module));
        add_own_shapes(&mut walk);
        walk.catalog
    }

    fn module(&mut self, name: String, text: &str) -> Rc<ModuleShapes> {
        Rc::clone(
            self.modules
                .entry(name)
                .or_insert_with_key(|name| Rc::new(ModuleShapes::read(name, text))),
        )
    }
}

impl CatalogWalk<'_> {
    fn add_module(&mut self, owner: Option<&str>, written: &ModulePath) {
        let Some(LocatedModule {
            source: ModuleSource::Text { name, text },
            owner: module_owner,
        }) = self.lookup.find(owner, written)
        else {
            return;
        };
        if self.visited.insert(name.clone()) {
            let module = self.reader.module(name, &text);
            self.add_shapes(module_owner.as_deref(), &module);
        }
    }

    fn add_shapes(&mut self, owner: Option<&str>, module: &ModuleShapes) {
        for path in &module.uses {
            self.add_module(owner, path);
        }
        for statement in &module.statements {
            self.catalog.add(statement);
        }
    }
}

impl ModuleShapes {
    fn read(name: &str, text: &str) -> Self {
        let mut sources = SourceMap::default();
        let source = sources.add(name, text);
        Reader::new(sources.file(source))
            .read()
            .map_or_else(|_| Self::default(), |forms| Self::of(&forms))
    }

    fn of(forms: &[Form]) -> Self {
        Self {
            statements: forms.iter().filter_map(ShapeStatement::of).collect(),
            uses: forms.iter().filter_map(Self::used_module).collect(),
        }
    }

    fn used_module(form: &Form) -> Option<ModulePath> {
        let [head, path, ..] = form.as_list()? else {
            return None;
        };
        let use_form = KernelForm::Definition(DefinitionForm::Use).name();
        (head.as_symbol()?.name() == use_form).then_some(())?;
        Some(ModulePath::new(path.as_symbol()?.name()))
    }
}
