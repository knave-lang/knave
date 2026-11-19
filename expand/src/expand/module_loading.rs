use std::rc::Rc;

use super::Expander;
use super::builtin_export::BuiltinExport;
use super::context::DefinitionContext;
use super::imported_members::{ImportedMembers, MemberTables};
use super::lint_records::UseFormText;
use super::local_file_diagnosis::LocalFileDiagnosis;
use super::module_body::PendingItem;
use super::module_load_site::ModuleLoadSite;
use super::problems::{ExpandProblems, MissingExportRemedy};
use super::shape::Shape;
use super::text_module_load::TextModuleLoad;
use super::text_module_record::TextModuleRecord;
use crate::core::{
    BindingId, BindingKind, Builtin, BuiltinModule, DefinitionForm, ImportedBindings,
    KernelConstants, KernelForm, KernelType, ModuleId, ModulePath, Namespace, Provenance,
    UnnamedPartPath, WrittenName, WrittenUse,
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
    const USE_HEAD_PARTS: usize = 2;

    pub(super) fn register_module_definition(
        &mut self,
        definition: DefinitionForm,
        form: Form,
        context: &mut DefinitionContext,
    ) -> Result<Option<PendingItem>, Diagnostic> {
        match definition {
            DefinitionForm::Use => Ok(self.import(&form, context)?.map(PendingItem::Use)),
            DefinitionForm::Export => Ok(Some(PendingItem::Export(form))),
            _ => unreachable!("only `use` and `export` reach this helper"),
        }
    }

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
                source: ModuleSource::Text { name, text, file },
                owner,
            }) => {
                let load = TextModuleLoad {
                    site: ModuleLoadSite {
                        path: path.clone(),
                        span,
                        owner,
                        importer: self.importing_owner(),
                    },
                    file,
                };
                if let TextModuleRecord::BindsTo { loaded, diagnostic } =
                    self.text_modules.record(load, self.sources)
                {
                    diagnostic
                        .into_iter()
                        .for_each(|error| self.problems.report(error));
                    let id = self
                        .modules
                        .find(&loaded)
                        .expect("a recorded first load is registered");
                    return self.loaded(id, path, span);
                }
                let source = self.sources.add(name, text);
                self.recorded_inputs.note_loaded(source);
                Ok(DeepRecursion::guard(|| {
                    self.load_source_module(path.clone(), &[source])
                }))
            }
            None => Err(self.local_file_diagnosis().missing_module(path, span)),
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

    fn import(
        &mut self,
        form: &Form,
        context: &DefinitionContext,
    ) -> Result<Option<WrittenUse>, Diagnostic> {
        let shape = Shape::new(KernelForm::Definition(DefinitionForm::Use), form);
        let items = shape.at_least(Self::USE_HEAD_PARTS)?;
        let written =
            UnnamedPartPath::check(shape.symbol(&items[1])?.name()).map_err(|unnamed| {
                self.local_file_diagnosis()
                    .unnamed_part_use(&unnamed, items[1].span())
            })?;
        let path = self.use_path(&written, items[1].span())?;
        let target = self.load_module(&path, items[1].span())?;
        let names = self.import_names(
            target,
            &path,
            &items[Self::USE_HEAD_PARTS..],
            &shape,
            context,
        )?;
        let introductions = self.tables.scopes.introductions();
        let written_in_source = items[..Self::USE_HEAD_PARTS]
            .iter()
            .all(|item| !introductions.carried_by_binder(item))
            && self
                .current_module
                .is_some_and(|module| ModuleSources::is_written(self.modules.get(module).path()));
        Ok(written_in_source.then(|| {
            let use_form = UseFormText::new(self.sources, form.span(), items[0].span());
            WrittenUse {
                span: form.span(),
                module: target,
                names: names
                    .into_iter()
                    .filter(|name| use_form.holds(name.span))
                    .collect(),
                listed: items.len() - Self::USE_HEAD_PARTS,
                provenance: Provenance::Written,
            }
        }))
    }

    fn import_names(
        &mut self,
        target: ModuleId,
        path: &ModulePath,
        names: &[Form],
        shape: &Shape<'_>,
        context: &DefinitionContext,
    ) -> Result<Vec<WrittenName>, Diagnostic> {
        let mut imported = Vec::with_capacity(names.len());
        for name in names {
            let mut binder = name.clone();
            context.strip_use_sites(&mut binder);
            let symbol = binder.as_symbol().ok_or_else(|| shape.malformed(name))?;
            match self.import_name(target, path, symbol, name.span()) {
                Ok(bindings) => {
                    let span = name.span();
                    imported.push(WrittenName { span, bindings });
                }
                Err(diagnostic) => self.problems.report(diagnostic),
            }
        }
        Ok(imported)
    }

    fn use_path(&mut self, written: &ModulePath, span: Span) -> Result<ModulePath, Diagnostic> {
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
        if let Some(unlisted) = self.module_sources.unlisted_library(owner, written) {
            return Err(ExpandProblems::unlisted_library(&unlisted, span));
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
        if !self.written_by_user(span) || self.reported_hidden_modules.contains(&shadow.hidden) {
            return;
        }
        let warning = ExpandProblems::test_module_hides_source_module(&shadow, span);
        self.problems.report_warning(warning);
        self.reported_hidden_modules.push(shadow.hidden);
    }

    fn written_by_user(&self, span: Span) -> bool {
        !self.sources.file(span.source()).is_generated()
    }

    fn importing_from_prelude(&self) -> bool {
        self.current_module.is_some_and(|module| {
            ModuleSources::is_embedded_prelude(self.modules.get(module).path())
        })
    }

    pub(super) fn expands_library_module(&self) -> bool {
        self.importing_owner().is_some()
    }

    fn local_file_diagnosis(&self) -> LocalFileDiagnosis<'_> {
        LocalFileDiagnosis::new(self.module_sources.local_files(self.importing_owner()))
    }

    fn importing_owner(&self) -> Option<Rc<str>> {
        let module = self.current_module?;
        self.text_modules.owner_of(self.modules.get(module).path())
    }

    fn loaded(&self, id: ModuleId, path: &ModulePath, span: Span) -> Result<ModuleId, Diagnostic> {
        if !self.modules.get(id).status().is_loading() {
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
        let base_scope = self.modules.get(module).scopes().base();
        if let Some(saved) = self.modules.get(base).exports().saved() {
            self.tables.table.import_prelude(base_scope, base, saved);
            return;
        }
        let mut scopes = ScopeSet::default();
        scopes.add(base_scope);
        let exports: Vec<(Rc<str>, Namespace, _)> =
            self.modules.get(base).exports().entries().collect();
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
    ) -> Result<ImportedBindings, Diagnostic> {
        let Some(bindings) = self.bind_exported_namespaces(target, symbol, span)? else {
            return Err(self.missing_export(target, path, symbol, span));
        };
        let tables = MemberTables::new(&self.tables.bindings, &self.tables.trait_methods);
        let members = ImportedMembers::new(self.modules.get(target), tables);
        let members = members.of(symbol.name(), bindings);
        self.bind_members(symbol, members, span)?;
        Ok(bindings)
    }

    fn bind_exported_namespaces(
        &mut self,
        target: ModuleId,
        symbol: &Symbol,
        span: Span,
    ) -> Result<Option<ImportedBindings>, Diagnostic> {
        let found = Namespace::IMPORTED
            .map(|namespace| self.modules.get(target).exported(symbol.name(), namespace));
        for (namespace, binding) in Namespace::IMPORTED.into_iter().zip(found) {
            if let Some(binding) = binding {
                self.bind_import(symbol, namespace, binding, span)?;
            }
        }
        Ok(ImportedBindings::found(found))
    }

    fn bind_members(
        &mut self,
        symbol: &Symbol,
        members: Vec<(Rc<str>, BindingId)>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        for (name, binding) in members {
            let member = Symbol::with_scopes(name, symbol.scopes().clone());
            self.bind_import(&member, Namespace::Value, binding, span)?;
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
            .entries()
            .map(|(name, _, _)| name)
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
            .any(|record| {
                record
                    .exports()
                    .entries()
                    .any(|(exported, _, _)| *exported == *name)
            })
    }

    fn create_builtin(&mut self, path: &ModulePath, builtin: BuiltinModule) -> ModuleId {
        let scopes = ModuleScopes::new(
            self.tables.scopes.fresh(ScopeKind::Module),
            self.tables.scopes.fresh(ScopeKind::Module),
        );
        let id = self.modules.register(path.clone(), scopes);
        for export in Self::builtin_exports(builtin) {
            let binding = self.tables.bindings.add(export.binding());
            for name in export.names() {
                self.modules
                    .get_mut(id)
                    .export(name.into(), export.namespace(), binding);
            }
        }
        self.modules.finish_loading(id);
        id
    }

    fn builtin_exports(builtin: BuiltinModule) -> Vec<BuiltinExport> {
        match builtin {
            BuiltinModule::Kernel => {
                KernelForm::all()
                    .map(|form| BuiltinExport::value(form.name(), BindingKind::KernelForm(form)))
                    .chain(KernelType::all().map(BuiltinExport::kernel_type))
                    .chain(KernelConstants::BOOLEANS.map(|(name, value)| {
                        BuiltinExport::value(name, BindingKind::Boolean(value))
                    }))
                    .collect()
            }
            BuiltinModule::Code | BuiltinModule::PreludeCode => Builtin::all()
                .filter(|each| each.module() == builtin)
                .map(|builtin| {
                    BuiltinExport::value(builtin.name(), BindingKind::CodeBuiltin(builtin))
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::expand::expanded_module::ExpandedModule;
    use crate::modules::ModuleSources;

    #[test]
    fn a_written_module_records_its_imports_and_the_prelude_none() {
        let program = ExpandedModule::of("(use code value-eq)\n(defn main () (println 1))");
        assert_eq!(program.entry_module().lint_facts.imports().len(), 1);
        assert!(
            program
                .modules
                .iter()
                .filter(|module| ModuleSources::is_embedded_prelude(&module.path))
                .all(|module| module.lint_facts.written_uses().is_empty()
                    && module.lint_facts.imports().is_empty()
                    && module.lint_facts.notes().is_empty())
        );
    }
}
