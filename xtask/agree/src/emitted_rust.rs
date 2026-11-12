use crate::subject_program::SubjectProgram;
use crate::timed_command::Finished;
use crate::top_level_definitions::TopLevelDefinitions;

pub struct EmittedRust {
    text: String,
    definitions: TopLevelDefinitions,
}

impl EmittedRust {
    pub(crate) fn of(finished: Finished, program: &SubjectProgram) -> Self {
        Self {
            text: String::from_utf8_lossy(&finished.stdout).into_owned(),
            definitions: program.definitions(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn knave_name(&self, rust_name: &str) -> String {
        self.definitions
            .named_in_rust(rust_name)
            .unwrap_or(rust_name)
            .to_string()
    }
}
