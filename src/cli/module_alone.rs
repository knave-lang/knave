use super::exit_status::ExitStatus;
use super::module_of_sources::ModuleOfSources;
use super::program_file::ProgramFile;
use super::terminal::Terminal;
use crate::core::ModulePath;
use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;

pub struct ModuleAlone<'f> {
    file: &'f ProgramFile,
}

impl<'f> ModuleAlone<'f> {
    const LOADING_SOURCE: &'static str = "<module>";
    const LOADING_FORM: &'static str = "(do)";

    pub fn new(file: &'f ProgramFile) -> Self {
        Self { file }
    }

    pub fn within<T>(
        &self,
        terminal: &mut Terminal,
        module: &str,
        work: impl FnOnce(ExpansionSession<'_>) -> Result<T, Diagnostics>,
    ) -> Result<T, ExitStatus> {
        self.within_silently(terminal, module, work)
            .map_err(|errors| terminal.source_report().report(&errors))
    }

    pub fn within_silently<T>(
        &self,
        terminal: &mut Terminal,
        module: &str,
        work: impl FnOnce(ExpansionSession<'_>) -> Result<T, Diagnostics>,
    ) -> Result<T, Diagnostics> {
        let sources = self.empty_entry(terminal);
        terminal
            .expansion()
            .sessions()
            .within_silently(&sources, |mut session| {
                Self::load(&mut session, module)?;
                work(session)
            })
    }

    pub fn load(session: &mut ExpansionSession<'_>, module: &str) -> Result<(), Diagnostics> {
        let source = session.add_source(Self::LOADING_SOURCE, Self::LOADING_FORM);
        let form = session.read(source)?.remove(0);
        session.expression_in(&ModulePath::new(module), form)?;
        Ok(())
    }

    fn empty_entry(&self, terminal: &mut Terminal) -> ModuleOfSources {
        let entry = terminal
            .sources()
            .add(self.file.path().display(), String::new());
        ModuleOfSources::single(self.file.clone(), entry)
    }
}
