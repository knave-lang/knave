use super::doc::ExpandedPage;
use super::doc_folder::WrittenPage;
use super::exit_status::ExitStatus;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use crate::core::{BindingId, Expr, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, Program};
use crate::modules::ModuleSources;
use crate::reader::{Form, StringLiteral};

pub struct DocumentedModules {
    file: ProgramFile,
    written: Vec<String>,
}

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
    const CALL_SOURCE: &'static str = "<documentation>";
    const LOADING_FORM: &'static str = "(do)";
    const NAMES_MACRO: &'static str = "documentation-names";
    const PAGE_MACRO: &'static str = "documentation-page";

    pub fn new(file: ProgramFile, written: Vec<String>) -> Self {
        Self { file, written }
    }

    pub fn public_prelude() -> Vec<String> {
        ModuleSources::PRELUDE_FILES
            .iter()
            .map(|(module, _)| ModulePath::new(*module))
            .filter(|module| !ModuleSources::is_internal(module))
            .map(|module| module.as_str().to_string())
            .collect()
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
        let entry = terminal.add_empty_source(self.file.path());
        let (program, named) = terminal.within_session(&self.file, entry, |mut session| {
            self.load_written(&mut session)?;
            let named = self.named_bindings(&mut session)?;
            let mut program = session.finish()?;
            for name in &named {
                program.check_also(name.binding);
            }
            Ok((program, named))
        })?;
        Self::typed_names(terminal, &program, named)
    }

    fn typed_names(
        terminal: &mut Terminal,
        program: &Program,
        named: Vec<NamedBinding>,
    ) -> Result<Vec<TypedName>, ExitStatus> {
        let typed = terminal.check_expanded(program)?;
        Ok(named
            .into_iter()
            .filter_map(|name| {
                let line = typed.signature(name.binding)?;
                Some(TypedName {
                    module: name.module,
                    name: name.name,
                    line,
                })
            })
            .collect())
    }

    fn load_written(&self, session: &mut ExpansionSession<'_>) -> Result<(), Diagnostics> {
        for module in &self.written {
            let form = Self::read(session, Self::LOADING_FORM)?;
            session.expression_in(&ModulePath::new(module.as_str()), form)?;
        }
        Ok(())
    }

    fn named_bindings(
        &self,
        session: &mut ExpansionSession<'_>,
    ) -> Result<Vec<NamedBinding>, Diagnostics> {
        let mut named = Vec::new();
        for module in &self.written {
            let call = format!("({} {module})", Self::NAMES_MACRO);
            let names = Self::expand(session, &call)?;
            let text = ExpandedPage::literal(&names).unwrap_or_default();
            named.extend(text.lines().filter_map(|name| {
                Some(NamedBinding {
                    module: module.clone(),
                    name: name.to_string(),
                    binding: session.module_value(module, name)?,
                })
            }));
        }
        Ok(named)
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
            self.written.join(" "),
            Self::public_prelude().join(" "),
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
