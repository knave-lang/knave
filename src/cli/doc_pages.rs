use super::doc::ExpandedPage;
use super::doc_folder::WrittenPage;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use crate::check::ProgramChecker;
use crate::core::{BindingId, Expr, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};
use crate::modules::{ModuleSources, PublicPrelude};
use crate::reader::{Form, StringLiteral};
use crate::types::TypedProgram;

pub struct DocumentedModules {
    file: ProgramFile,
    written: Vec<String>,
    linked: LinkedModules,
}

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

impl DocumentedModules {
    pub const CALL_SOURCE: &'static str = "<documentation>";
    const LOADING_FORM: &'static str = "(do)";
    const NAMES_MACRO: &'static str = "documentation-names";
    const PAGE_MACRO: &'static str = "documentation-page";

    pub fn new(file: ProgramFile, written: Vec<String>) -> Self {
        let linked = LinkedModules {
            written: written.join(" "),
            prelude: PublicPrelude::paths().join(" "),
        };
        Self {
            file,
            written,
            linked,
        }
    }

    pub fn pages(&self, terminal: &mut Terminal) -> Result<Vec<WrittenPage>, ExitStatus> {
        let types = self.types(terminal)?;
        let entry = terminal.add_empty_source(self.file.path());
        let texts = terminal.within_session(&self.file, entry, |mut session| {
            self.load_written(&mut session)?;
            self.written
                .iter()
                .map(|module| Self::expand(&mut session, &self.page_call(module, &types)))
                .collect::<Result<Vec<Expr>, Diagnostics>>()
        })?;
        self.written
            .iter()
            .zip(texts)
            .map(|(module, text)| {
                Ok(WrittenPage {
                    module: module.clone(),
                    text: ExpandedPage::string(&text, Self::PAGE_MACRO)?,
                })
            })
            .collect()
    }

    fn types(&self, terminal: &mut Terminal) -> Result<Vec<TypedName>, ExitStatus> {
        let mut types = Vec::new();
        let mut whole_program = None;
        for module in &self.written {
            match self.own_types(terminal, module)? {
                Some(own) => types.extend(own),
                None => {
                    let whole = match whole_program.take() {
                        Some(whole) => whole,
                        None => self.whole_program_types(terminal)?,
                    };
                    types.extend(Self::of_module(&whole, module));
                    whole_program = Some(whole);
                }
            }
        }
        Ok(types)
    }

    fn of_module(types: &[TypedName], module: &str) -> Vec<TypedName> {
        types
            .iter()
            .filter(|typed| typed.module == module)
            .cloned()
            .collect()
    }

    fn own_types(
        &self,
        terminal: &mut Terminal,
        module: &str,
    ) -> Result<Option<Vec<TypedName>>, ExitStatus> {
        let entry = terminal.add_empty_source(self.file.path());
        let (program, named) = terminal.within_session(&self.file, entry, |mut session| {
            Self::load(&mut session, module)?;
            let named = Self::named_bindings(&mut session, module)?;
            Ok((Self::checked_also(session.finish()?, &named), named))
        })?;
        Ok(ProgramChecker::check(&program)
            .ok()
            .map(|typed| Self::typed_names(&typed, named)))
    }

    fn whole_program_types(&self, terminal: &mut Terminal) -> Result<Vec<TypedName>, ExitStatus> {
        let entry = terminal.add_empty_source(self.file.path());
        let (program, named) = terminal.within_session(&self.file, entry, |mut session| {
            self.load_written(&mut session)?;
            let mut named = Vec::new();
            for module in &self.written {
                named.extend(Self::named_bindings(&mut session, module)?);
            }
            Ok((Self::checked_also(session.finish()?, &named), named))
        })?;
        let typed = terminal.check_showing(&program, |_| false)?;
        Ok(Self::typed_names(&typed, named))
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
