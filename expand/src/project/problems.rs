use super::LibrarySource;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label, Suggestion};
use crate::language::Language;
use crate::source::Span;

pub struct ProjectProblems;

impl ProjectProblems {
    const PROJECT_EXAMPLE: &str = "(project my-program)";

    pub fn missing_project_form(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_PROJECT_FORM,
            format!(
                "`{}` must hold a `project` form",
                Language::project_file_name()
            ),
            Label::new(span, "expected `(project <name> ...)` here"),
        )
        .with_help(format!("start the file with `{}`", Self::PROJECT_EXAMPLE))
    }

    pub fn extra_project_form(extra: Span, project: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::EXTRA_PROJECT_FORM,
            format!(
                "`{}` holds a form after its `project` form",
                Language::project_file_name()
            ),
            Label::new(extra, "this form is outside the `project` form"),
        )
        .with_label(Label::new(project, "the `project` form ends here"))
        .with_help("move this form inside the `project` form, or remove it")
    }

    pub fn invalid_project_name(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::INVALID_PROJECT_NAME,
            format!("`{name}` is not a project name"),
            Label::new(span, "expected a kebab-case name"),
        )
        .with_note("the name names the build directory and the program")
        .with_help("use lowercase letters, digits and single dashes, as `my-program`")
    }

    pub fn reserved_library_name(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::RESERVED_LIBRARY_NAME,
            format!("`{name}` cannot name a library"),
            Label::new(span, "this name is reserved"),
        )
        .with_note("`prelude`, `kernel` and `code` name the compiler's own modules")
        .with_help("give the library another name")
    }

    pub fn invalid_library_name(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::INVALID_LIBRARY_NAME,
            format!("`{name}` is not a library name"),
            Label::new(span, "expected a kebab-case name"),
        )
        .with_note("`use` reads a library's name as the first part of a module path")
        .with_help("use lowercase letters, digits and single dashes, as `my-library`")
    }

    pub fn library_at_two_sources(name: &str, span: Span, first: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LIBRARY_AT_TWO_SOURCES,
            format!("library `{name}` comes from two sources"),
            Label::new(span, "this source differs"),
        )
        .with_label(Label::new(first, "from the source given here"))
        .with_help(format!("point every `{name}` entry at one source"))
    }

    pub fn library_without_project_file(name: &str, span: Span, project_file: &str) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LIBRARY_WITHOUT_PROJECT_FILE,
            format!("library `{name}` has no project file at `{project_file}`"),
            Label::new(span, "this library's `:path` names no project"),
        )
        .with_help("point `:path` at the library's directory, which holds its project file")
    }

    pub fn library_without_root_module(
        name: &str,
        span: Span,
        root_file: &str,
        project_file: &str,
    ) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LIBRARY_WITHOUT_ROOT_MODULE,
            format!("library `{name}` has no root module `{root_file}`"),
            Label::new(span, "this library's `:lib` names no file"),
        )
        .with_help(format!("fix `:lib` in `{project_file}`, or add the file"))
    }

    pub fn unavailable_git_library(name: &str, span: Span, reason: &str) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::GIT_LIBRARY_UNAVAILABLE,
            format!("cannot fetch git library `{name}`"),
            Label::new(span, "this library's commit could not be checked out"),
        )
        .with_note(format!("git says: {reason}"))
        .with_help("check the library's `:git` and `:rev`, and that the repository is reachable")
    }

    pub fn repository_like_an_option(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::INVALID_GIT_REPOSITORY,
            "`:git` cannot start with `-`",
            Label::new(span, "git would read this as an option"),
        )
        .with_help("write the repository's URL or path, as `https://example.com/keymap.git`")
    }

    pub fn path_outside_checkout(name: &str, key: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::PATH_OUTSIDE_CHECKOUT,
            format!("library `{name}` names a `{key}` outside its checkout"),
            Label::new(span, "this git library's project file reaches outside it"),
        )
        .with_note("a library fetched from git may only name files inside its own checkout")
        .with_help(format!(
            "ask the library's authors to use a relative `{key}` without `..`"
        ))
    }

    pub fn crate_path_outside_checkout(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::PATH_OUTSIDE_CHECKOUT,
            format!("crate `{name}` names a `:path` outside its library's checkout"),
            Label::new(span, "this git library's project file reaches outside it"),
        )
        .with_note("a library fetched from git may only name files inside its own checkout")
        .with_help("ask the library's authors to use a relative `:path` without `..`")
    }

    pub fn crate_requests_conflict(name: &str, span: Span, first: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CRATE_REQUESTS_CONFLICT,
            format!("crate `{name}` has two requests that cannot merge"),
            Label::new(span, "this request"),
        )
        .with_label(Label::new(first, "conflicts with this one"))
        .with_note(
            "the program builds one copy of each crate, and only plain versions that are \
             semver-compatible merge",
        )
        .with_help(format!(
            "make the `{name}` entries ask for compatible plain versions"
        ))
    }

    pub fn crates_unavailable_to(command: &str, span: Span, label: String) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CRATES_UNAVAILABLE_TO_COMMAND,
            format!("`{command}` cannot use crate dependencies yet"),
            Label::new(span, label),
        )
        .with_help("use `build`, `run` or `test` for a program with crates for now")
    }

    pub fn missing_project_name(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_PROJECT_NAME,
            "the project has no name",
            Label::new(span, "expected a name after `project`"),
        )
        .with_help(format!("name the project, as `{}`", Self::PROJECT_EXAMPLE))
    }

    pub fn unknown_key(key: &str, span: Span, closest: Option<&str>) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::UNKNOWN_PROJECT_KEY,
            format!("unknown key `:{key}`"),
            Label::new(span, "not a key of this form"),
        );
        match closest {
            Some(closest) => diagnostic.with_suggestion(
                "a key with a similar name exists",
                Suggestion::new(span, format!(":{closest}")),
            ),
            None => diagnostic,
        }
    }

    pub fn missing_value(key: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISSING_PROJECT_VALUE,
            format!("`:{key}` has no value"),
            Label::new(span, "expected a value after this key"),
        )
        .with_help(format!("write the value right after `:{key}`"))
    }

    pub fn mismatched_value(key: &str, expected: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISMATCHED_PROJECT_VALUE,
            format!("`:{key}` takes {expected}"),
            Label::new(span, format!("expected {expected}")),
        )
    }

    pub fn unquoted_feature(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MISMATCHED_PROJECT_VALUE,
            "a feature is a string",
            Label::new(span, "expected a string"),
        )
        .with_suggestion(
            "quote the feature's name",
            Suggestion::new(span, format!("\"{name}\"")),
        )
    }

    pub fn unknown_section(section: &str, span: Span, closest: Option<&str>) -> Diagnostic {
        let diagnostic = Diagnostic::error(
            DiagnosticCode::UNKNOWN_PROJECT_SECTION,
            format!("unknown section `{section}`"),
            Label::new(span, "not a section of a project"),
        );
        match closest {
            Some(closest) => diagnostic.with_suggestion(
                "a section with a similar name exists",
                Suggestion::new(span, closest),
            ),
            None => diagnostic
                .with_help("the sections are `libraries`, `dependencies` and `dev-dependencies`"),
        }
    }

    pub fn unexpected_form(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNEXPECTED_PROJECT_FORM,
            "expected a key or a section",
            Label::new(span, "this is neither a `:key value` pair nor a section"),
        )
        .with_help(format!(
            "give a value after a key, as `:entry \"src/main.{}\"`",
            Language::EXTENSION
        ))
    }

    pub fn unexpected_entry_form(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNEXPECTED_PROJECT_FORM,
            "expected a key",
            Label::new(span, "this is not a `:key value` pair"),
        )
        .with_help("an entry is a name, then an optional version, then `:key value` pairs")
    }

    pub fn duplicate_entry(name: &str, span: Span, first: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::DUPLICATE_PROJECT_ENTRY,
            format!("`{name}` is given more than once"),
            Label::new(span, "given again here"),
        )
        .with_label(Label::new(first, "first given here"))
        .with_help("remove one of them")
    }

    pub fn malformed_dependency(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::MALFORMED_DEPENDENCY,
            "expected a dependency",
            Label::new(span, "a dependency is a list that starts with a name"),
        )
        .with_help("write a dependency as `(serde \"1.0\")` or `(hints :path \"../hints\")`")
    }

    pub fn versioned_library(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::VERSIONED_LIBRARY,
            format!("library `{name}` cannot take a version"),
            Label::new(span, "a library is found by its source, not its version"),
        )
        .with_help("remove the version")
    }

    pub fn library_without_source(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::LIBRARY_WITHOUT_SOURCE,
            format!("library `{name}` has no source"),
            Label::new(span, "this library needs `:path` or `:git`"),
        )
        .with_help(format!(
            "write `({name} :path \"../{name}\")`, or `:git` with a `:rev`"
        ))
    }

    pub fn conflicting_sources(name: &str, span: Span, other: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::CONFLICTING_LIBRARY_SOURCES,
            format!("library `{name}` has two sources"),
            Label::new(span, "a git source"),
        )
        .with_label(Label::new(other, "conflicts with this one"))
        .with_help("keep `:path` alone, or `:git` with `:rev`")
    }

    pub fn git_without_revision(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::GIT_LIBRARY_WITHOUT_REVISION,
            format!("git library `{name}` has no `:rev`"),
            Label::new(span, "this repository needs a commit"),
        )
        .with_help(format!(
            "add `:rev` with the commit's full {}-character hash",
            LibrarySource::FULL_COMMIT_LENGTH
        ))
    }

    pub fn short_revision(span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::SHORT_REVISION,
            "`:rev` must name a full commit",
            Label::new(
                span,
                format!(
                    "expected {} hexadecimal digits",
                    LibrarySource::FULL_COMMIT_LENGTH
                ),
            ),
        )
        .with_note("git fetches a commit only by its full hash")
        .with_help("write the commit's full hash, as `git rev-parse <commit>` prints it")
    }

    pub fn unversioned_crate(name: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNVERSIONED_CRATE,
            format!("crate `{name}` has no version"),
            Label::new(span, "this crate needs a version or a `:path`"),
        )
        .with_help(format!(
            "write the version after the name, as `({name} \"1.0\")`"
        ))
    }

    pub fn unknown_edition(written: &str, span: Span) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::UNKNOWN_EDITION,
            format!("edition {written} does not exist"),
            Label::new(span, "not an edition of this compiler"),
        )
        .with_suggestion(
            format!("the only edition is {}", Language::EDITION),
            Suggestion::new(span, Language::EDITION.to_string()),
        )
    }
}
