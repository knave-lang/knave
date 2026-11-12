use rustc_hash::FxHashMap;

use super::doc::ExpandedPage;
use super::doc_folder::WrittenPage;
use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::module_of_sources::ModuleOfSources;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use crate::check::ProgramChecker;
use crate::core::{BindingId, Expr, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};
use crate::format::ModuleLookup;
use crate::modules::{ModuleSources, PublicPrelude};
use crate::ownership::EnforcedOwnership;
use crate::reader::{Form, StringLiteral};
use crate::types::TypedProgram;

pub struct DocumentedModules {
    file: ProgramFile,
    entry: EntryProgram,
    written: Vec<String>,
    linked: LinkedModules,
}

pub struct EntryProgram {
    file: ProgramFile,
    text: String,
}

type ModuleTypes = FxHashMap<String, Vec<TypedName>>;

struct LinkedModules {
    written: String,
    prelude: String,
}

#[derive(Clone)]
struct TypedName {
    module: String,
    name: String,
    line: String,
}

struct NamedBinding {
    module: String,
    name: String,
    binding: BindingId,
}

impl EntryProgram {
    pub fn new(file: ProgramFile, text: String) -> Self {
        Self { file, text }
    }
}

impl DocumentedModules {
    pub const CALL_SOURCE: &'static str = "<documentation>";
    const LOADING_FORM: &'static str = "(do)";
    const NAMES_MACRO: &'static str = "documentation-names";
    const PAGE_MACRO: &'static str = "documentation-page";

    pub fn new(file: ProgramFile, entry: EntryProgram, written: Vec<String>) -> Self {
        let linked = LinkedModules {
            written: written.join(" "),
            prelude: PublicPrelude::paths().join(" "),
        };
        Self {
            file,
            entry,
            written,
            linked,
        }
    }

    pub fn pages(&self, terminal: &mut Terminal) -> Result<Vec<WrittenPage>, ExitStatus> {
        let types = self.types(terminal)?;
        let entry = terminal
            .sources()
            .add(self.file.path().display(), String::new());
        let (texts, warnings) = terminal.expansion().within_session(
            &ModuleOfSources::single(self.file.clone(), entry),
            |mut session| {
                self.load_written(&mut session)?;
                let texts = self
                    .written
                    .iter()
                    .map(|module| Self::expand(&mut session, &self.page_call(module, &types)))
                    .collect::<Result<Vec<Expr>, Diagnostics>>()?;
                Ok((texts, session.finish()?.warnings))
            },
        )?;
        terminal.source_report().print(&warnings);
        let mut formatter = EmbeddedPrelude::source_formatter();
        let lookup = ModuleLookup::within_project(self.file.module_sources());
        self.written
            .iter()
            .zip(texts)
            .map(|(module, text)| {
                let page = ExpandedPage::string(&text, Self::PAGE_MACRO)?;
                Ok(WrittenPage {
                    module: module.clone(),
                    text: formatter.lay_out_page(&page, module, &lookup),
                })
            })
            .collect()
    }

    fn types(&self, terminal: &mut Terminal) -> Result<Vec<TypedName>, ExitStatus> {
        let own_checks = self
            .written
            .iter()
            .map(|module| self.own_types(terminal, module))
            .collect::<Result<Vec<_>, ExitStatus>>()?;
        let entry_types = if own_checks.iter().any(Result::is_err) {
            Some(self.entry_program_types(terminal)?)
        } else {
            None
        };
        let mut types = Vec::new();
        for (module, own) in self.written.iter().zip(own_checks) {
            types.extend(own.or_else(|own_errors| {
                Self::of_module(entry_types.as_ref(), module)
                    .ok_or_else(|| terminal.source_report().report(&own_errors))
            })?);
        }
        Ok(types)
    }

    fn of_module(entry_types: Option<&ModuleTypes>, module: &str) -> Option<Vec<TypedName>> {
        entry_types?.get(module).cloned()
    }

    fn own_types(
        &self,
        terminal: &mut Terminal,
        module: &str,
    ) -> Result<Result<Vec<TypedName>, Diagnostics>, ExitStatus> {
        let entry = terminal
            .sources()
            .add(self.file.path().display(), String::new());
        let (program, named) = terminal.expansion().within_session(
            &ModuleOfSources::single(self.file.clone(), entry),
            |mut session| {
                Self::load(&mut session, module)?;
                let named = Self::named_bindings(&mut session, module)?;
                Ok((Self::checked_also(session.finish()?, &named), named))
            },
        )?;
        Ok(ProgramChecker::check::<EnforcedOwnership>(&program)
            .map(|typed| Self::typed_names(&typed, named)))
    }

    fn entry_program_types(&self, terminal: &mut Terminal) -> Result<ModuleTypes, ExitStatus> {
        let entry = &self.entry;
        let source = terminal
            .sources()
            .add(entry.file.path().display(), entry.text.clone());
        let (program, loaded, named) = terminal.expansion().within_session(
            &ModuleOfSources::single(entry.file.clone(), source),
            |mut session| {
                let loaded = self.loaded_written(&session);
                let mut named = Vec::new();
                for module in &loaded {
                    named.extend(Self::named_bindings(&mut session, module)?);
                }
                Ok((Self::checked_also(session.finish()?, &named), loaded, named))
            },
        )?;
        let typed = terminal.checking().check_silently(&program)?;
        Ok(Self::by_module(loaded, Self::typed_names(&typed, named)))
    }

    fn loaded_written(&self, session: &ExpansionSession<'_>) -> Vec<String> {
        self.written
            .iter()
            .filter(|module| session.loads(&ModulePath::new(module.as_str())))
            .cloned()
            .collect()
    }

    fn by_module(loaded: Vec<String>, typed: Vec<TypedName>) -> ModuleTypes {
        let mut types: ModuleTypes = loaded
            .into_iter()
            .map(|module| (module, Vec::new()))
            .collect();
        for name in typed {
            types.entry(name.module.clone()).or_default().push(name);
        }
        types
    }

    fn checked_also(mut program: Program, named: &[NamedBinding]) -> Program {
        for name in named {
            program.check_also(name.binding);
        }
        program
    }

    fn typed_names(typed: &TypedProgram, named: Vec<NamedBinding>) -> Vec<TypedName> {
        named
            .into_iter()
            .filter_map(|name| {
                let line = typed.signature(name.binding)?;
                Some(TypedName {
                    module: name.module,
                    name: name.name,
                    line,
                })
            })
            .collect()
    }

    fn load_written(&self, session: &mut ExpansionSession<'_>) -> Result<(), Diagnostics> {
        for module in &self.written {
            Self::load(session, module)?;
        }
        Ok(())
    }

    fn load(session: &mut ExpansionSession<'_>, module: &str) -> Result<(), Diagnostics> {
        let form = Self::read(session, Self::LOADING_FORM)?;
        session.expression_in(&ModulePath::new(module), form)?;
        Ok(())
    }

    fn named_bindings(
        session: &mut ExpansionSession<'_>,
        module: &str,
    ) -> Result<Vec<NamedBinding>, Diagnostics> {
        let call = format!("({} {module})", Self::NAMES_MACRO);
        let names = Self::expand(session, &call)?;
        let text = ExpandedPage::literal(&names).unwrap_or_default();
        Ok(text
            .lines()
            .filter_map(|name| {
                Some(NamedBinding {
                    module: module.to_string(),
                    name: name.to_string(),
                    binding: session.module_value(module, name)?,
                })
            })
            .collect())
    }

    fn page_call(&self, module: &str, types: &[TypedName]) -> String {
        let entries: Vec<String> = types
            .iter()
            .filter(|typed| typed.module == module)
            .map(|typed| format!("({} {})", typed.name, StringLiteral::quoted(&typed.line)))
            .collect();
        format!(
            "({} {module} ({}) ({}) ({}))",
            Self::PAGE_MACRO,
            self.linked.written,
            self.linked.prelude,
            entries.join(" ")
        )
    }

    fn expand(session: &mut ExpansionSession<'_>, call: &str) -> Result<Expr, Diagnostics> {
        let form = Self::read(session, call)?;
        session.expression_in(&ModulePath::new(ModuleSources::DOCUMENTATION), form)
    }

    fn read(session: &mut ExpansionSession<'_>, text: &str) -> Result<Form, Diagnostics> {
        let source = session.add_source(Self::CALL_SOURCE, text);
        let mut forms = session.read(source)?;
        Ok(forms.remove(0))
    }
}
