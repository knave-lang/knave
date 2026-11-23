use std::path::PathBuf;

use super::compiler_version_problems::CompilerVersionProblems;
use super::dependency_reader::DependencyReader;
use super::exported_task_reader::ExportedTaskReader;
use super::fields::{FieldScanner, Fields, ScannedForms};
use super::lint_reader::LintReader;
use super::name::ProjectName;
use super::problems::ProjectProblems;
use super::release_version::ReleaseVersion;
use super::section::Section;
use super::vocabulary::ProjectVocabulary;
use super::{CompilerVersion, Project, ProjectModules};
use crate::diagnostic::Diagnostics;
use crate::language::Language;
use crate::reader::Form;
use crate::source::Span;

pub struct ProjectReader<'v> {
    problems: Diagnostics,
    vocabulary: &'v ProjectVocabulary,
}

struct ProjectForm<'f> {
    head: &'f Form,
    name: Option<&'f Form>,
    body: &'f [Form],
}

impl<'v> ProjectReader<'v> {
    const KEYS: &'static [&'static str] = &["version", "entry", "lib", "edition", Language::NAME];
    const DEFAULT_VERSION: &'static str = "0.1.0";
    const DEFAULT_ENTRY: &'static str = "src/main";
    const DEFAULT_LIBRARY_ROOT: &'static str = "src/lib";

    pub fn new(vocabulary: &'v ProjectVocabulary) -> Self {
        Self {
            problems: Diagnostics::default(),
            vocabulary,
        }
    }

    pub fn read(mut self, forms: &[Form], file: Span) -> Result<Project, Diagnostics> {
        let project = self.project(forms, file);
        Self::outcome(project, self.problems)
    }

    pub fn recorded_version(mut self, forms: &[Form], file: Span) -> Option<CompilerVersion> {
        let form = self.project_form(forms, file)?;
        let scanned = self.scanned(&form);
        self.compiler_version(&scanned.fields)
    }

    fn outcome(project: Option<Project>, problems: Diagnostics) -> Result<Project, Diagnostics> {
        match project {
            Some(project) => problems.into_result(project),
            None => Err(problems),
        }
    }

    fn project(&mut self, forms: &[Form], file: Span) -> Option<Project> {
        let form = self.project_form(forms, file)?;
        let name = self.name(&form);
        let scanned = self.scanned(&form);
        let mut sections = SectionTable::default();
        scanned
            .others
            .iter()
            .for_each(|other| self.read_section(other, &mut sections));
        self.with_fields(name, &scanned.fields, sections)
    }

    fn scanned<'f>(&mut self, form: &ProjectForm<'f>) -> ScannedForms<'f> {
        FieldScanner::new(Self::KEYS, &mut self.problems).scan(form.body)
    }

    fn project_form<'f>(&mut self, forms: &'f [Form], file: Span) -> Option<ProjectForm<'f>> {
        let Some(first) = forms.first() else {
            self.problems.push(ProjectProblems::missing_project_form(
                file.start(),
                self.vocabulary.head(),
            ));
            return None;
        };
        if let Some(extra) = forms.get(1) {
            self.problems.push(ProjectProblems::extra_project_form(
                extra.span(),
                first.span(),
                self.vocabulary.head(),
            ));
        }
        let form = ProjectForm::of(first, self.vocabulary.head());
        if form.is_none() {
            self.problems.push(ProjectProblems::missing_project_form(
                first.span(),
                self.vocabulary.head(),
            ));
        }
        form
    }

    fn name(&mut self, form: &ProjectForm<'_>) -> Option<String> {
        let span = form.name.unwrap_or(form.head).span();
        let Some(name) = form.name.and_then(DependencyReader::bare_name) else {
            self.problems.push(ProjectProblems::missing_project_name(
                span,
                self.vocabulary.head(),
            ));
            return None;
        };
        if let Err(problem) = ProjectName::check(name, span) {
            self.problems.push(problem);
            return None;
        }
        Some(name.to_string())
    }

    fn read_section(&mut self, form: &Form, sections: &mut SectionTable) {
        let Some((head, entries)) = Self::section_parts(form) else {
            return self
                .problems
                .push(ProjectProblems::unexpected_form(form.span()));
        };
        let Some(section) = self.section_named(head) else {
            return;
        };
        if let Some(first) = sections.heads.iter().find(|seen| seen.section == section) {
            return self.problems.push(ProjectProblems::duplicate_entry(
                self.vocabulary.section_name(section),
                head.span(),
                first.span,
            ));
        }
        sections.heads.push(SectionHead {
            section,
            span: head.span(),
        });
        match section {
            Section::Lints => LintReader::new(self.vocabulary, &mut self.problems)
                .read(entries, &mut sections.lints),
            Section::Dependency(kind) => {
                DependencyReader::new(kind, &mut self.problems).read(entries, sections)
            }
            Section::ExportedTasks => {
                ExportedTaskReader::new(self.vocabulary.section_name(section), &mut self.problems)
                    .read(entries, &mut sections.exported_tasks)
            }
        }
    }

    fn section_parts(form: &Form) -> Option<(&Form, &[Form])> {
        let (head, entries) = form.as_list()?.split_first()?;
        head.as_symbol()?;
        Some((head, entries))
    }

    fn section_named(&mut self, head: &Form) -> Option<Section> {
        let name = head
            .as_symbol()
            .map(|symbol| symbol.name())
            .unwrap_or_default();
        let section = self.vocabulary.section_named(name);
        if section.is_none() {
            self.problems.push(ProjectProblems::unknown_section(
                name,
                head.span(),
                self.vocabulary,
            ));
        }
        section
    }

    fn with_fields(
        &mut self,
        name: Option<String>,
        fields: &Fields<'_>,
        sections: SectionTable,
    ) -> Option<Project> {
        let version = fields
            .string("version", &mut self.problems)
            .unwrap_or_else(|| Self::DEFAULT_VERSION.to_string());
        let modules = self.modules(fields);
        let edition = self.edition(fields);
        let compiler_version = self.compiler_version(fields);
        Some(Project {
            name: name?,
            version,
            modules,
            edition,
            compiler_version,
            libraries: sections.libraries,
            dependencies: sections.dependencies,
            dev_dependencies: sections.dev_dependencies,
            lints: sections.lints,
            exported_tasks: sections.exported_tasks,
        })
    }

    fn compiler_version(&mut self, fields: &Fields<'_>) -> Option<CompilerVersion> {
        let (text, span) = fields.located_string(Language::NAME, &mut self.problems)?;
        let Some(version) = ReleaseVersion::parse(&text) else {
            let mismatch = ProjectProblems::mismatched_value(
                Language::NAME,
                CompilerVersionProblems::EXPECTED_VERSION,
                span,
            );
            self.problems
                .push(CompilerVersionProblems::malformed_version(mismatch));
            return None;
        };
        Some(CompilerVersion::new(version, span))
    }

    fn modules(&mut self, fields: &Fields<'_>) -> ProjectModules {
        let written_entry = self.written_module_file(fields, "entry");
        let entry_written = written_entry.is_some();
        ProjectModules {
            entry: written_entry.unwrap_or_else(|| Self::default_module_file(Self::DEFAULT_ENTRY)),
            library_root: self
                .written_module_file(fields, "lib")
                .unwrap_or_else(|| Self::default_module_file(Self::DEFAULT_LIBRARY_ROOT)),
            entry_written,
        }
    }

    fn written_module_file(&mut self, fields: &Fields<'_>, key: &str) -> Option<PathBuf> {
        fields.string(key, &mut self.problems).map(PathBuf::from)
    }

    fn default_module_file(default_module: &str) -> PathBuf {
        PathBuf::from(format!("{default_module}.{}", Language::EXTENSION))
    }

    fn edition(&mut self, fields: &Fields<'_>) -> i128 {
        let Some((edition, span)) = fields.whole_number("edition", &mut self.problems) else {
            return Language::EDITION;
        };
        if edition != Language::EDITION {
            self.problems
                .push(ProjectProblems::unknown_edition(&edition.to_string(), span));
        }
        edition
    }
}

impl<'f> ProjectForm<'f> {
    fn of(form: &'f Form, project_head: &str) -> Option<Self> {
        let (head, parts) = form.as_list()?.split_first()?;
        if head.as_symbol()?.name() != project_head {
            return None;
        }
        let name = parts
            .first()
            .filter(|first| FieldScanner::key_of(first).is_none());
        let body = &parts[usize::from(name.is_some())..];
        Some(Self { head, name, body })
    }
}

struct SectionHead {
    section: Section,
    span: Span,
}

#[derive(Default)]
pub struct SectionTable {
    heads: Vec<SectionHead>,
    pub libraries: Vec<super::LibraryDependency>,
    pub dependencies: Vec<super::CrateDependency>,
    pub dev_dependencies: Vec<super::CrateDependency>,
    pub lints: super::LintTable,
    pub exported_tasks: Vec<super::ExportedTask>,
}
