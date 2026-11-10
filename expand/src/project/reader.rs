use std::path::PathBuf;
use std::rc::Rc;

use super::Project;
use super::dependency_reader::{DependencyReader, Section};
use super::fields::{FieldScanner, Fields};
use super::name::ProjectName;
use super::problems::ProjectProblems;
use crate::diagnostic::{Diagnostics, SimilarName};
use crate::language::Language;
use crate::reader::Form;
use crate::source::Span;

pub struct ProjectReader {
    problems: Diagnostics,
}

struct ProjectForm<'f> {
    head: &'f Form,
    name: Option<&'f Form>,
    body: &'f [Form],
}

impl ProjectReader {
    pub const HEAD: &str = "project";
    const KEYS: &[&str] = &["version", "entry", "lib", "edition"];
    const DEFAULT_VERSION: &str = "0.1.0";
    const DEFAULT_ENTRY: &str = "src/main";
    const DEFAULT_LIBRARY_ROOT: &str = "src/lib";

    pub fn read(forms: &[Form], file: Span) -> Result<Project, Diagnostics> {
        let mut reader = Self {
            problems: Diagnostics::default(),
        };
        let project = reader.project(forms, file);
        match project {
            Some(project) => reader.problems.into_result(project),
            None => Err(reader.problems),
        }
    }

    fn project(&mut self, forms: &[Form], file: Span) -> Option<Project> {
        let form = self.project_form(forms, file)?;
        let name = self.name(&form);
        let scanned = FieldScanner::new(Self::KEYS, &mut self.problems).scan(form.body);
        let mut sections = SectionTable::default();
        scanned
            .others
            .iter()
            .for_each(|other| self.read_section(other, &mut sections));
        self.with_fields(name, &scanned.fields, sections)
    }

    fn project_form<'f>(&mut self, forms: &'f [Form], file: Span) -> Option<ProjectForm<'f>> {
        let Some(first) = forms.first() else {
            self.problems
                .push(ProjectProblems::missing_project_form(file.start()));
            return None;
        };
        if let Some(extra) = forms.get(1) {
            self.problems.push(ProjectProblems::extra_project_form(
                extra.span(),
                first.span(),
            ));
        }
        let form = ProjectForm::of(first);
        if form.is_none() {
            self.problems
                .push(ProjectProblems::missing_project_form(first.span()));
        }
        form
    }

    fn name(&mut self, form: &ProjectForm<'_>) -> Option<String> {
        let span = form.name.unwrap_or(form.head).span();
        let Some(name) = form.name.and_then(DependencyReader::bare_name) else {
            self.problems
                .push(ProjectProblems::missing_project_name(span));
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
                section.name(),
                head.span(),
                first.span,
            ));
        }
        sections.heads.push(SectionHead {
            section,
            span: head.span(),
        });
        DependencyReader::new(section, &mut self.problems).read(entries, sections);
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
        let section = Section::named(name);
        if section.is_none() {
            let candidates: Vec<Rc<str>> = Section::ALL
                .iter()
                .map(|section| Rc::from(section.name()))
                .collect();
            let closest = SimilarName::closest(name, &candidates);
            self.problems.push(ProjectProblems::unknown_section(
                name,
                head.span(),
                closest.as_deref(),
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
        let entry = self.module_file(fields, "entry", Self::DEFAULT_ENTRY);
        let library_root = self.module_file(fields, "lib", Self::DEFAULT_LIBRARY_ROOT);
        let edition = self.edition(fields);
        Some(Project {
            name: name?,
            version,
            entry,
            library_root,
            edition,
            libraries: sections.libraries,
            dependencies: sections.dependencies,
            dev_dependencies: sections.dev_dependencies,
        })
    }

    fn module_file(&mut self, fields: &Fields<'_>, key: &str, default_module: &str) -> PathBuf {
        fields
            .string(key, &mut self.problems)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{default_module}.{}", Language::EXTENSION)))
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
    fn of(form: &'f Form) -> Option<Self> {
        let (head, parts) = form.as_list()?.split_first()?;
        if head.as_symbol()?.name() != ProjectReader::HEAD {
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
}
