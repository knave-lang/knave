use super::binding_owners::BindingOwners;
use super::definition_reach::{DefinitionReach, UnreachedDefinition};
use super::linted_module::LintedModule;
use super::module_use::ModuleUse;
use super::rule_target::RuleTarget;
use super::spanned_forms::SpannedForms;
use crate::core::{BindingId, DefinitionForm, Import, ImportedName, KernelForm};
use crate::diagnostic::Diagnostic;
use crate::expand::Program;
use crate::interpret::ListNesting;
use crate::reader::Form;
use crate::source::{SourceMap, Span};

pub struct ModuleTable<'p> {
    program: &'p Program,
    sources: &'p SourceMap,
    owners: BindingOwners,
    reach: DefinitionReach,
}

impl<'p> ModuleTable<'p> {
    const MACRO_ONLY: &'static str = "macro";
    const VALUE: &'static str = "value";

    pub fn new(program: &'p Program, sources: &'p SourceMap, reach: DefinitionReach) -> Self {
        Self {
            program,
            sources,
            owners: BindingOwners::of(program),
            reach,
        }
    }

    pub fn target(&self, linted: &LintedModule) -> Result<Option<RuleTarget>, Diagnostic> {
        let module = linted.module();
        let first_item = module.items.first().map(|item| item.span);
        let recorded = module.lint_facts.imports();
        let Some(span) = first_item.or_else(|| recorded.first().map(|import| import.span)) else {
            return Ok(None);
        };
        let used = ModuleUse::of(self.program, &self.owners, linted);
        let definitions = self
            .reach
            .unreached_in(module.id)
            .iter()
            .map(|definition| self.definition(definition, span))
            .collect();
        let imports = recorded
            .iter()
            .map(|import| self.import(import, &used))
            .collect();
        let parts = vec![
            SpannedForms::list(definitions, span),
            SpannedForms::list(imports, span),
        ];
        let table = ListNesting::form_list(parts, span).map_err(|deep| deep.at(span))?;
        Ok(Some(RuleTarget::for_module(
            table,
            module.path.as_str().into(),
            span,
        )))
    }

    fn definition(&self, definition: &UnreachedDefinition, span: Span) -> Form {
        let readers = definition
            .readers
            .iter()
            .map(|reader| self.binder(*reader, span))
            .collect();
        SpannedForms::list(
            vec![
                self.binder(definition.binding, span),
                SpannedForms::list(readers, span),
            ],
            span,
        )
    }

    fn import(&self, import: &Import, used: &ModuleUse) -> Form {
        let span = import.span;
        let unread = import
            .names
            .iter()
            .filter(|name| !used.refers_to(name))
            .map(|name| self.unread_name(name, used))
            .collect();
        let parts = vec![
            SpannedForms::kernel_head(KernelForm::Definition(DefinitionForm::Use), span),
            SpannedForms::number(import.names.len() as i128, span),
            SpannedForms::list(unread, span),
        ];
        SpannedForms::list(parts, span)
    }

    fn unread_name(&self, name: &ImportedName, used: &ModuleUse) -> Form {
        let brings = if used.brings_only_macros(name) {
            Self::MACRO_ONLY
        } else {
            Self::VALUE
        };
        SpannedForms::list(
            vec![
                SpannedForms::symbol(self.written(name.span), name.span),
                SpannedForms::symbol(brings, name.span),
            ],
            name.span,
        )
    }

    fn written(&self, span: Span) -> &str {
        &self.sources.file(span.source()).text()[span.range()]
    }

    fn binder(&self, binding: BindingId, span: Span) -> Form {
        let info = self.program.bindings.get(binding);
        SpannedForms::symbol(info.name(), info.span().unwrap_or(span))
    }
}
