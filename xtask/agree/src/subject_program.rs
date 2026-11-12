use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::form_census::FormCensus;
use crate::scratch_directory::ScratchDirectory;
use crate::seed::Seed;
use crate::top_level_definitions::TopLevelDefinitions;
use crate::verdict::Verdict;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectProgram {
    pub name: String,
    pub seed: Option<Seed>,
    pub files: Vec<(PathBuf, String)>,
    pub entry: PathBuf,
    pub arguments: Vec<String>,
    pub stdin: Vec<u8>,
    pub forms: FormCensus,
}

impl SubjectProgram {
    pub fn of_entry(name: String, entry: PathBuf, text: String) -> Self {
        let forms = FormCensus::of_text(&text);
        Self {
            name,
            seed: None,
            files: vec![(entry.clone(), text)],
            entry,
            arguments: Vec::new(),
            stdin: Vec::new(),
            forms,
        }
    }

    pub fn generated(seed: Seed, entry: PathBuf, text: String) -> Self {
        Self {
            seed: Some(seed),
            ..Self::of_entry(seed.to_string(), entry, text)
        }
    }

    pub fn with_entry_text(&self, text: String) -> Self {
        let files = self
            .files
            .iter()
            .map(|(path, old)| {
                let kept = if path == &self.entry { &text } else { old };
                (path.clone(), kept.clone())
            })
            .collect();
        Self {
            files,
            forms: FormCensus::of_text(&text),
            ..self.clone()
        }
    }

    pub fn judged_in_scratch(
        &self,
        judgement: impl FnOnce(&ScratchDirectory) -> Verdict,
    ) -> Verdict {
        match self.written_to_scratch() {
            Ok(directory) => judgement(&directory),
            Err(error) => Verdict::Unsupported(format!("cannot write the program: {error}")),
        }
    }

    pub fn entry_text(&self) -> &str {
        self.text_of(&self.entry).unwrap_or_default()
    }

    pub fn definitions(&self) -> TopLevelDefinitions {
        TopLevelDefinitions::of_text(self.entry_text())
    }

    pub fn text_of(&self, file: &Path) -> Option<&str> {
        self.files
            .iter()
            .find(|(path, _)| path == file)
            .map(|(_, text)| text.as_str())
    }

    fn written_to_scratch(&self) -> io::Result<ScratchDirectory> {
        let directory = ScratchDirectory::create()?;
        self.write_into(directory.path())?;
        Ok(directory)
    }

    fn write_into(&self, directory: &Path) -> io::Result<()> {
        for (path, text) in &self.files {
            let destination = directory.join(path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(destination, text)?;
        }
        Ok(())
    }
}
