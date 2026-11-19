use super::definition_reach::{DefinitionReach, UnreachedDefinition};
use super::linted_module::LintedModule;
use super::rule_target::RuleTarget;
use super::spanned_forms::SpannedForms;
use std::rc::Rc;

use crate::core::{BindingId, BindingKind, DefinitionForm, Import, ImportedName, KernelForm};
use crate::diagnostic::Diagnostic;
use crate::expand::Program;
use crate::interpret::ListNesting;
use crate::reader::Form;
use crate::source::{SourceMap, Span};

pub struct ModuleTable<'p> {
    program: &'p Program,
    sources: &'p SourceMap,
    reach: DefinitionReach,
}

impl<'p> ModuleTable<'p> {
    const MACRO_ONLY: &'static str = "macro";
    const MODULE_READ: &'static str = "module-read";
    const MEMBERS: &'static str = "members";

    pub fn new(program: &'p Program, sources: &'p SourceMap, reach: DefinitionReach) -> Self {
        Self {
            program,
            sources,
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
        let definitions = self
            .reach
            .unreached_in(module.id)
            .iter()
            .map(|definition| self.definition(definition, span))
            .collect();
        let imports = recorded.iter().map(|import| self.import(import)).collect();
        let notes = module
            .lint_facts
            .notes()
            .iter()
            .map(|(key, notes)| Self::notes(key, notes, span))
            .collect();
        let parts = vec![
            SpannedForms::list(definitions, span),
            SpannedForms::list(imports, span),
            SpannedForms::list(notes, span),
        ];
        let table = ListNesting::form_list(parts, span).map_err(|deep| deep.at(span))?;
        Ok(Some(RuleTarget::for_module(
            table,
            module.path.as_str().into(),
            span,
        )))
    }

    fn notes(key: &str, notes: &[Form], span: Span) -> Form {
        let mut parts = vec![SpannedForms::symbol(key, span)];
        parts.extend(notes.iter().cloned());
        SpannedForms::list(parts, span)
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

    fn import(&self, import: &Import) -> Form {
        let span = import.span;
        let unread = import
            .names
            .iter()
            .enumerate()
            .filter(|(_, name)| !name.used)
            .map(|(position, name)| self.unread_name(name, import.members_of(position)))
            .collect();
        let mut parts = vec![
            SpannedForms::kernel_head(KernelForm::Definition(DefinitionForm::Use), span),
            SpannedForms::number(import.listed as i128, span),
            SpannedForms::list(unread, span),
        ];
        if import.module_read {
            parts.push(SpannedForms::symbol(Self::MODULE_READ, span));
        }
        SpannedForms::list(parts, span)
    }

    fn unread_name<'i>(
        &self,
        name: &ImportedName,
        members: impl Iterator<Item = &'i Rc<str>>,
    ) -> Form {
        let written = &name.written;
        let span = written.span;
        let mut parts = vec![SpannedForms::symbol(written.text(self.sources), span)];
        if self.brings_only_macros(name) {
            parts.push(SpannedForms::symbol(Self::MACRO_ONLY, span));
        }
        let mut listed = vec![SpannedForms::symbol(Self::MEMBERS, span)];
        listed.extend(members.map(|member| SpannedForms::symbol(member, span)));
        if listed.len() > 1 {
            parts.push(SpannedForms::list(listed, span));
        }
        SpannedForms::list(parts, span)
    }

    fn brings_only_macros(&self, name: &ImportedName) -> bool {
        name.written.bindings.iter().all(|binding| {
            matches!(
                self.program.bindings.get(binding).kind(),
                BindingKind::Macro(_)
            )
        })
    }

    fn binder(&self, binding: BindingId, span: Span) -> Form {
        let info = self.program.bindings.get(binding);
        SpannedForms::symbol(info.name(), info.span().unwrap_or(span))
    }
}
