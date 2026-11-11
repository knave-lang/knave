use std::rc::Rc;

use super::Expander;
use super::context::DefinitionContext;
use super::problems::{ExpandProblems, MissingExportRemedy};
use super::shape::Shape;
use crate::core::{
    BindingId, BindingInfo, BindingKind, Builtin, BuiltinModule, DefinitionForm, KernelConstants,
    KernelForm, KernelType, ModuleId, ModulePath, Namespace,
};
use crate::diagnostic::Diagnostic;
use crate::diagnostic::SimilarName;
use crate::interpret::Warnings;
use crate::modules::{LocatedModule, ModuleScopes, ModuleSource, ModuleSources};
use crate::reader::{Form, Reader, Symbol};
use crate::scope::{Phase, ScopeKind, ScopeSet};
use crate::source::{SourceId, Span};
use crate::stack::DeepRecursion;

impl Expander<'_> {
    const NAMESPACES: [Namespace; 2] = [Namespace::Value, Namespace::Type];

    pub(super) fn load_module(
        &mut self,
        path: &ModulePath,
        span: Span,
    ) -> Result<ModuleId, Diagnostic> {
        if let Some(id) = self.modules.find(path) {
            return self.loaded(id, path, span);
        }
        if let Some(prelude) = self
            .saved_prelude
            .take_if(|_| path.as_str() == ModuleSources::PRELUDE)
        {
            return Ok(prelude.install(self));
        }
        if ModuleSources::is_reserved(path) {
            return Err(ExpandProblems::reserved_module(path, span));
        }
        match self.module_sources.locate(path) {
            Some(LocatedModule {
                source: ModuleSource::Builtin(builtin),
                ..
            }) => Ok(self.create_builtin(path, builtin)),
            Some(LocatedModule {
                source: ModuleSource::Text { name, text },
                owner,
            }) => {
                self.note_library_module(path, owner);
                let source = self.sources.add(name, text);
                self.recorded_inputs.note_loaded(source);
                Ok(DeepRecursion::guard(|| {
                    self.load_source_module(path.clone(), &[source])
                }))
            }
            None => Err(self.missing_module(path, span)),
        }
    }

    fn missing_module(&self, path: &ModulePath, span: Span) -> Diagnostic {
        match self.module_sources.unreachable_file(path) {
            Some(file) => ExpandProblems::unreachable_module(path, &file, span),
            None => ExpandProblems::unknown_module(path, span),
        }
    }

    pub(super) fn load_source_module(
        &mut self,
        path: ModulePath,
        sources: &[SourceId],
    ) -> ModuleId {
        let id = self.register_source_module(path.clone());
        let forms = self.read_sources(sources);
        self.import_base(id, &path, Self::module_start(sources));
        let record = self.modules.get(id);
        let scopes = record.scopes();
        let scoped = forms
            .into_iter()
            .map(|mut form| {
                form.add_scope(scopes.base());
                form.add_scope(scopes.body());
                form
            })
            .collect();
        self.expand_module_body(id, scoped);
        self.modules.finish_loading(id);
        id
    }

    fn read_sources(&mut self, sources: &[SourceId]) -> Vec<Form> {
        let mut sources = sources.iter();
        let mut forms = sources
            .next()
            .map(|first| self.read_module(*first))
            .unwrap_or_default();
        for source in sources {
            forms.extend(self.read_module(*source));
        }
        forms
    }

    fn module_start(sources: &[SourceId]) -> Span {
        let first = sources.first().expect("a source module has a source");
        Span::new(*first, 0..0)
    }

    pub(super) fn register_source_module(&mut self, path: ModulePath) -> ModuleId {
        let scopes = ModuleScopes::new(
            self.tables.scopes.fresh(ScopeKind::Module),
            self.tables.scopes.fresh(ScopeKind::Module),
        );
        self.modules.register(path, scopes)
    }

    pub(super) fn import(
        &mut self,
        form: &Form,
        context: &DefinitionContext,
    ) -> Result<(), Diagnostic> {
        let shape = Shape::new(KernelForm::Definition(DefinitionForm::Use), form);
        let items = shape.at_least(2)?;
        let written = ModulePath::new(shape.symbol(&items[1])?.name());
        let path = self.resolve_use(&written, items[1].span())?;
        let target = self.load_module(&path, items[1].span())?;
        self.import_names(target, &path, &items[2..], &shape, context)
    }

    fn import_names(
        &mut self,
        target: ModuleId,
        path: &ModulePath,
        names: &[Form],
        shape: &Shape<'_>,
        context: &DefinitionContext,
    ) -> Result<(), Diagnostic> {
        for name in names {
            let mut binder = name.clone();
            context.strip_use_sites(&mut binder);
            let symbol = binder.as_symbol().ok_or_else(|| shape.malformed(name))?;
            if let Err(diagnostic) = self.import_name(target, path, symbol, name.span()) {
                self.problems.report(diagnostic);
            }
        }
        Ok(())
    }

    fn resolve_use(&mut self, written: &ModulePath, span: Span) -> Result<ModulePath, Diagnostic> {
        if ModuleSources::is_internal(written) && !self.importing_from_prelude() {
            return Err(ExpandProblems::internal_module(written, span));
        }
        let owner = self.importing_owner();
        let owner = owner.as_deref();
        if let Some(clash) = self
            .module_sources
            .local_module_beside_library(owner, written)
        {
            return Err(ExpandProblems::local_module_beside_library(&clash, span));
        }
        if let Some(library) = self.module_sources.unlisted_library(owner, written) {
            return Err(ExpandProblems::unlisted_library(library, span));
        }
        self.report_hidden_local_module(owner, written, span);
        self.report_hidden_source_module(owner, written, span);
        let path = self.module_sources.resolve(owner, written);
        match self.module_sources.module_outside_checkout(&path) {
            Some(file) => Err(ExpandProblems::module_outside_checkout(&path, &file, span)),
            None => Ok(path),
        }
    }

    fn report_hidden_local_module(
        &mut self,
        owner: Option<&str>,
        written: &ModulePath,
        span: Span,
    ) {
        let Some(local) = self.module_sources.hidden_local_module(owner, written) else {
            return;
        };
        if self.reported_hidden_modules.contains(&local) {
            return;
        }
        let warning = ExpandProblems::library_hides_local_module(written, &local, span);
        self.problems.report_warning(warning);
        self.reported_hidden_modules.push(local);
    }

    fn report_hidden_source_module(
        &mut self,
        owner: Option<&str>,
        written: &ModulePath,
        span: Span,
    ) {
        let Some(shadow) = self.module_sources.hidden_source_module(owner, written) else {
            return;
        };
        let use_site = self.sources.file(span.source()).name();
        if !ModuleSources::is_written_file(use_site)
            || self.reported_hidden_modules.contains(&shadow.hidden)
        {
            return;
        }
        let warning = ExpandProblems::test_module_hides_source_module(&shadow, span);
        self.problems.report_warning(warning);
        self.reported_hidden_modules.push(shadow.hidden);
    }

    fn note_library_module(&mut self, path: &ModulePath, owner: Option<Rc<str>>) {
        if let Some(owner) = owner {
            self.library_modules.push((path.clone(), owner));
        }
    }

    fn importing_from_prelude(&self) -> bool {
        self.current_module.is_some_and(|module| {
            ModuleSources::is_embedded_prelude(self.modules.get(module).path())
        })
    }

    pub(super) fn is_library_module(&self, path: &ModulePath) -> bool {
        self.library_modules
            .iter()
            .any(|(loaded, _)| loaded == path)
    }

    fn importing_owner(&self) -> Option<Rc<str>> {
        let module = self.current_module?;
        let path = self.modules.get(module).path();
        self.library_modules
            .iter()
            .find(|(loaded, _)| loaded == path)
            .map(|(_, owner)| Rc::clone(owner))
    }

    fn loaded(&self, id: ModuleId, path: &ModulePath, span: Span) -> Result<ModuleId, Diagnostic> {
        if !self.modules.get(id).is_loading() {
            return Ok(id);
        }
        let mut cycle: Vec<String> = self
            .modules
            .loading_paths()
            .iter()
            .map(ToString::to_string)
            .collect();
        cycle.push(path.to_string());
        Err(ExpandProblems::import_cycle(path, span, cycle))
    }

    fn read_module(&mut self, source: SourceId) -> Vec<Form> {
        match Reader::new(self.sources.file(source)).read() {
            Ok(forms) => forms,
            Err(diagnostics) => {
                self.problems.report_all(diagnostics);
                Vec::new()
            }
        }
    }

    fn import_base(&mut self, module: ModuleId, path: &ModulePath, span: Span) {
        let base_name = if ModuleSources::is_prelude(path) {
            ModuleSources::KERNEL
        } else {
            ModuleSources::PRELUDE
        };
        match self.load_module(&ModulePath::new(base_name), span) {
            Ok(base) => self.import_everything(module, base),
            Err(diagnostic) => self.problems.report(diagnostic),
        }
    }

    fn import_everything(&mut self, module: ModuleId, base: ModuleId) {
        let mut scopes = ScopeSet::default();
        scopes.add(self.modules.get(module).scopes().base());
        let exports: Vec<(Rc<str>, Namespace, _)> = self
            .modules
            .get(base)
            .exports()
            .map(|(name, namespace, binding)| (Rc::clone(name), namespace, binding))
            .collect();
        for (name, namespace, binding) in exports {
            let symbol = Symbol::with_scopes(name, scopes.clone());
            self.tables
                .table
                .bind(&symbol, namespace, binding, Phase::Runtime)
                .expect("a module's exports have distinct names");
        }
    }

    fn import_name(
        &mut self,
        target: ModuleId,
        path: &ModulePath,
        symbol: &Symbol,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if !self.bind_exported_namespaces(target, symbol, span)? {
            return Err(self.missing_export(target, path, symbol, span));
        }
        self.import_members(target, symbol, span)
    }

    fn bind_exported_namespaces(
        &mut self,
        target: ModuleId,
        symbol: &Symbol,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        let mut found = false;
        for namespace in Self::NAMESPACES {
            if let Some(binding) = self.modules.get(target).exported(symbol.name(), namespace) {
                self.bind_import(symbol, namespace, binding, span)?;
                found = true;
            }
        }
        Ok(found)
    }

    fn import_members(
        &mut self,
        target: ModuleId,
        symbol: &Symbol,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Some(binding) = self
            .modules
            .get(target)
            .exported(symbol.name(), Namespace::Type)
        else {
            return Ok(());
        };
        if matches!(self.kind(binding), BindingKind::Enum(_)) {
            self.import_enum_constructors(target, symbol, span)?;
        }
        if matches!(self.kind(binding), BindingKind::Trait(_)) {
            self.import_trait_methods(target, symbol, binding, span)?;
        }
        Ok(())
    }

    fn import_enum_constructors(
        &mut self,
        target: ModuleId,
        symbol: &Symbol,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let prefix = format!("{}/", symbol.name());
        let constructors: Vec<(Rc<str>, BindingId)> = self
            .modules
            .get(target)
            .exports()
            .filter(|(name, namespace, _)| {
                *namespace == Namespace::Value && name.starts_with(&prefix)
            })
            .map(|(name, _, binding)| (Rc::clone(name), binding))
            .collect();
        for (name, binding) in constructors {
            let constructor = Symbol::with_scopes(name, symbol.scopes().clone());
            self.bind_import(&constructor, Namespace::Value, binding, span)?;
        }
        Ok(())
    }

    fn import_trait_methods(
        &mut self,
        target: ModuleId,
        symbol: &Symbol,
        trait_binding: BindingId,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let Some(trait_methods) = self.tables.trait_methods.get(&trait_binding) else {
            return Ok(());
        };
        let methods: Vec<BindingId> = trait_methods
            .methods
            .iter()
            .map(|(binding, _)| *binding)
            .collect();
        for method in methods {
            let name: Rc<str> = self.tables.bindings.get(method).name().into();
            if self.modules.get(target).exported(&name, Namespace::Value) != Some(method) {
                continue;
            }
            let imported = Symbol::with_scopes(Rc::clone(&name), symbol.scopes().clone());
            self.bind_import(&imported, Namespace::Value, method, span)?;
        }
        Ok(())
    }

    fn missing_export(
        &self,
        target: ModuleId,
        path: &ModulePath,
        symbol: &Symbol,
        span: Span,
    ) -> Diagnostic {
        let exported: Vec<Rc<str>> = self
            .modules
            .get(target)
            .exports()
            .map(|(name, _, _)| Rc::clone(name))
            .collect();
        let similar = SimilarName::closest(symbol.name(), &exported);
        let remedy = self.missing_export_remedy(path, symbol.name());
        ExpandProblems::missing_export(path, symbol.name(), span, similar, remedy)
    }

    fn missing_export_remedy(&self, path: &ModulePath, name: &str) -> MissingExportRemedy {
        if ModuleSources::is_compiler_provided(path) {
            return MissingExportRemedy::NoHelp;
        }
        if !ModuleSources::is_embedded_prelude(path) {
            return MissingExportRemedy::AddToExports;
        }
        if self.internal_modules_export(name) {
            return MissingExportRemedy::UseADefiner;
        }
        MissingExportRemedy::NoHelp
    }

    fn internal_modules_export(&self, name: &str) -> bool {
        self.modules
            .records()
            .filter(|record| ModuleSources::is_internal(record.path()))
            .filter(|record| !ModuleSources::is_compiler_provided(record.path()))
            .any(|record| record.exports().any(|(exported, _, _)| **exported == *name))
    }

    fn create_builtin(&mut self, path: &ModulePath, builtin: BuiltinModule) -> ModuleId {
        let scopes = ModuleScopes::new(
            self.tables.scopes.fresh(ScopeKind::Module),
            self.tables.scopes.fresh(ScopeKind::Module),
        );
        let id = self.modules.register(path.clone(), scopes);
        for (name, namespace, kind) in Self::builtin_exports(builtin) {
            let binding = self
                .tables
                .bindings
                .add(BindingInfo::new(name.into(), kind, None));
            self.modules
                .get_mut(id)
                .export(name.into(), namespace, binding);
        }
        self.modules.finish_loading(id);
        id
    }

    fn builtin_exports(builtin: BuiltinModule) -> Vec<(&'static str, Namespace, BindingKind)> {
        match builtin {
            BuiltinModule::Kernel => KernelForm::all()
                .map(|form| (form.name(), Namespace::Value, BindingKind::KernelForm(form)))
                .chain(KernelType::all().map(|kernel_type| {
                    (
                        kernel_type.name(),
                        Namespace::Type,
                        BindingKind::KernelType(kernel_type),
                    )
                }))
                .chain(
                    KernelConstants::BOOLEANS
                        .map(|(name, value)| (name, Namespace::Value, BindingKind::Boolean(value))),
                )
                .collect(),
            BuiltinModule::Code | BuiltinModule::PreludeCode => Builtin::all()
                .filter(|each| each.module() == builtin)
                .map(|builtin| {
                    (
                        builtin.name(),
                        Namespace::Value,
                        BindingKind::CodeBuiltin(builtin),
                    )
                })
                .collect(),
        }
    }
}
