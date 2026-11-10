use super::crate_section::CrateSection;
use super::exit_status::ExitStatus;
use super::project_file_edit::ProjectFileEdit;
use super::standard_error::StandardError;
use super::temporary_directory::TemporaryDirectory;
use super::terminal::Terminal;
use crate::build::{CrateResolver, ResolveError, ResolvedCrate};
use crate::project::CrateEntry;

pub struct AddCommand {
    name: String,
    features: Vec<String>,
    section: CrateSection,
}

struct CrateNaming;

impl AddCommand {
    const COMMAND: &str = "add";

    pub fn new(name: String, features: Vec<String>, section: CrateSection) -> Self {
        Self {
            name,
            features,
            section,
        }
    }

    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        match self.add(terminal) {
            Ok(()) => ExitStatus::Success,
            Err(status) => status,
        }
    }

    fn add(&self, terminal: &Terminal) -> Result<(), ExitStatus> {
        self.check_names()?;
        let edit = ProjectFileEdit::of_working_directory(terminal, Self::COMMAND)?;
        Self::refuse_path(&edit, &self.name)?;
        let resolved = self.resolved()?;
        Self::refuse_path(&edit, &resolved.name)?;
        let listed = edit.listed_crate(self.section, &resolved.name);
        let entry = CrateEntry {
            features: self.features_with(listed.map(|dependency| dependency.features.clone())),
            name: resolved.name,
            version: resolved.version,
        };
        edit.apply(terminal, |editor| {
            Some(editor.with_crate(self.section.section(), &entry))
        })?;
        Ok(())
    }

    fn check_names(&self) -> Result<(), ExitStatus> {
        if !CrateNaming::is_crate_name(&self.name) {
            return Err(Self::report_usage(&format!(
                "`{}` is not a crate name",
                self.name
            )));
        }
        match self
            .features
            .iter()
            .find(|feature| !CrateNaming::is_feature_name(feature))
        {
            Some(feature) => Err(Self::report_usage(&format!(
                "`{feature}` is not a feature name"
            ))),
            None => Ok(()),
        }
    }

    fn refuse_path(edit: &ProjectFileEdit, name: &str) -> Result<(), ExitStatus> {
        let Some((section, dependency)) = CrateSection::ALL.into_iter().find_map(|section| {
            edit.listed_crate(section, name)
                .filter(|dependency| dependency.path.is_some())
                .map(|dependency| (section, dependency))
        }) else {
            return Ok(());
        };
        Err(Self::report_usage(&format!(
            "`{name}` is listed with `:path`\nhelp: run `{remove} {name}` first to switch it to a version",
            name = dependency.name,
            remove = section.command("remove")
        )))
    }

    fn features_with(&self, listed: Option<Vec<String>>) -> Vec<String> {
        let mut features = listed.unwrap_or_default();
        for feature in &self.features {
            if !features.contains(feature) {
                features.push(feature.clone());
            }
        }
        features
    }

    fn resolved(&self) -> Result<ResolvedCrate, ExitStatus> {
        TemporaryDirectory::create()
            .map_err(ResolveError::Unwritable)
            .and_then(|scratch| {
                CrateResolver::new(scratch.path().to_path_buf()).resolve(&self.name, &self.features)
            })
            .map_err(|error| match error {
                ResolveError::CargoMissing(error) => Self::report_environment(&format!(
                    "cannot run cargo, which finds the version of a crate to add: {error}"
                )),
                ResolveError::Unwritable(error) => Self::report_environment(&format!(
                    "cannot write a scratch manifest for cargo: {error}"
                )),
                ResolveError::Cargo(message) => Self::report_cargo(&message),
            })
    }

    fn report_usage(message: &str) -> ExitStatus {
        StandardError::write_line(format_args!("error: {message}"));
        ExitStatus::UsageError
    }

    fn report_cargo(message: &str) -> ExitStatus {
        StandardError::write_line(format_args!("{message}"));
        ExitStatus::Environment
    }

    fn report_environment(message: &str) -> ExitStatus {
        StandardError::write_line(format_args!("error: {message}"));
        ExitStatus::Environment
    }
}

impl CrateNaming {
    const NAME_MARKS: [char; 2] = ['-', '_'];
    const FEATURE_MARKS: [char; 4] = ['-', '_', '+', '.'];
    const DEPENDENCY_FEATURE: char = '/';
    const FEATURE_START: char = '_';

    fn is_crate_name(name: &str) -> bool {
        name.starts_with(|first: char| first.is_ascii_alphabetic())
            && name.chars().all(|character| {
                character.is_ascii_alphanumeric() || Self::NAME_MARKS.contains(&character)
            })
    }

    fn is_feature_name(feature: &str) -> bool {
        let parts: Vec<&str> = feature.split(Self::DEPENDENCY_FEATURE).collect();
        parts.len() <= 2 && parts.iter().all(|part| Self::is_feature_part(part))
    }

    fn is_feature_part(part: &str) -> bool {
        part.starts_with(|first: char| {
            first.is_ascii_alphanumeric() || first == Self::FEATURE_START
        }) && part.chars().all(|character| {
            character.is_ascii_alphanumeric() || Self::FEATURE_MARKS.contains(&character)
        })
    }
}
