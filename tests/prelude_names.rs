use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use knave::core::ModulePath;
use knave::expand::Expander;
use knave::modules::ModuleSources;
use knave::reader::{Form, FormKind, Reader};
use knave::source::SourceMap;

struct NamingException {
    name: &'static str,
    reason: &'static str,
}

impl NamingException {
    const ALL: &[NamingException] = &[
        Self::of("<", "a comparison operator returns `bool` without `?`"),
        Self::of("<=", "a comparison operator returns `bool` without `?`"),
        Self::of("=", "a comparison operator returns `bool` without `?`"),
        Self::of(">", "a comparison operator returns `bool` without `?`"),
        Self::of(">=", "a comparison operator returns `bool` without `?`"),
        Self::of("not=", "a comparison operator returns `bool` without `?`"),
        Self::of("not", "the logical operator returns `bool` without `?`"),
        Self::of("set!", "a macro: no value type shows its `mut` place"),
        Self::of("sort!", "a macro: no value type shows its `mut` vector"),
        Self::of(
            "push-at!",
            "a kernel form: no value type shows its `mut` target",
        ),
        Self::of(
            "set-at!",
            "a kernel form: no value type shows its `mut` place",
        ),
    ];

    const fn of(name: &'static str, reason: &'static str) -> Self {
        Self { name, reason }
    }

    fn excuses(name: &str) -> bool {
        Self::ALL.iter().any(|exception| exception.name == name)
    }
}

struct PreludeNaming {
    compiler: Compiler,
}

impl PreludeNaming {
    const PREDICATE_SUFFIX: char = '?';
    const MUTATION_SUFFIX: char = '!';
    const ENUM_CASE_SEPARATOR: char = '/';

    fn load() -> Self {
        Self { compiler: Compiler }
    }

    fn broken_names(&self) -> Vec<String> {
        let names = self.exported_names();
        let values = ValueProbe::of(names.clone()).values();
        let types = self.compiler.inferred_types(&values);
        names
            .iter()
            .filter(|name| !NamingException::excuses(name))
            .filter_map(|name| Self::problem(name, Self::type_of(&types, name)))
            .collect()
    }

    fn exported_names(&self) -> Vec<String> {
        self.compiler
            .exported_names()
            .into_iter()
            .filter(|name| !Self::is_enum_case(name))
            .collect()
    }

    fn is_enum_case(name: &str) -> bool {
        name.split_once(Self::ENUM_CASE_SEPARATOR)
            .is_some_and(|(enum_name, _)| !enum_name.is_empty())
    }

    fn type_of<'a>(types: &'a [(String, FunctionShape)], name: &str) -> Option<&'a FunctionShape> {
        types
            .iter()
            .find(|(typed, _)| typed == name)
            .map(|(_, shape)| shape)
    }

    fn problem(name: &str, shape: Option<&FunctionShape>) -> Option<String> {
        let shape = shape.copied().unwrap_or_default();
        let marked_predicate = name.ends_with(Self::PREDICATE_SUFFIX);
        let marked_mutation = name.ends_with(Self::MUTATION_SUFFIX);
        if marked_predicate != shape.returns_bool {
            return Some(format!("{name}: `?` does not match a `bool` result"));
        }
        if marked_mutation != shape.takes_mut {
            return Some(format!("{name}: `!` does not match a `mut` parameter"));
        }
        None
    }
}

#[derive(Clone, Copy, Default)]
struct FunctionShape {
    returns_bool: bool,
    takes_mut: bool,
}

impl FunctionShape {
    const WHERE: &str = "where";
    const FUNCTION: &str = "fn";
    const MUT: &str = "mut";
    const BOOL: &str = "bool";

    fn of(type_text: &str) -> Self {
        let mut sources = SourceMap::default();
        let source = sources.add("<type>", type_text);
        let forms = Reader::new(sources.file(source))
            .read()
            .expect("an inferred type reads");
        Self::of_form(&forms[0])
    }

    fn of_form(form: &Form) -> Self {
        match Self::headed(form) {
            Some((Self::WHERE, [_, body])) => Self::of_form(body),
            Some((Self::FUNCTION, [parameters, result])) => Self {
                returns_bool: Self::symbol(result) == Some(Self::BOOL),
                takes_mut: Self::any_mut(parameters),
            },
            _ => Self::default(),
        }
    }

    fn any_mut(parameters: &Form) -> bool {
        parameters
            .as_list()
            .unwrap_or_default()
            .iter()
            .any(|parameter| matches!(Self::headed(parameter), Some((Self::MUT, _))))
    }

    fn headed(form: &Form) -> Option<(&str, &[Form])> {
        let (head, rest) = form.as_list()?.split_first()?;
        Some((Self::symbol(head)?, rest))
    }

    fn symbol(form: &Form) -> Option<&str> {
        match form.kind() {
            FormKind::Symbol(symbol) => Some(symbol.name()),
            _ => None,
        }
    }
}

struct ValueProbe {
    names: Vec<String>,
}

impl ValueProbe {
    const MODULE: &str = "probe";
    const FILE: &str = "probe.knv";

    fn of(names: Vec<String>) -> Self {
        Self { names }
    }

    fn values(self) -> Vec<String> {
        let failing = self.failing_lines();
        self.names
            .into_iter()
            .enumerate()
            .filter(|(line, _)| !failing.contains(line))
            .map(|(_, name)| name)
            .collect()
    }

    fn failing_lines(&self) -> BTreeSet<usize> {
        let text = self.probe_text();
        let mut sources = SourceMap::default();
        let entry = sources.add(Self::FILE, text.clone());
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let module_sources = ModuleSources::new(directory.clone(), directory);
        let Err(diagnostics) = Expander::new(&mut sources, module_sources)
            .expand(ModulePath::new(Self::MODULE), entry)
        else {
            return BTreeSet::new();
        };
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.primary().span())
            .filter(|span| span.source() == entry)
            .map(|span| text[..span.range().start].matches('\n').count())
            .collect()
    }

    fn probe_text(&self) -> String {
        self.names
            .iter()
            .enumerate()
            .map(|(position, name)| format!("(def probe-{position} (infer {name}))\n"))
            .collect()
    }
}

struct Compiler;

impl Compiler {
    const PATH: &str = env!("CARGO_BIN_EXE_knave");
    const EMPTY_MODULE: &str = "";
    const MODULE_FILE: &str = "probe.knv";
    const LISTING_FORM: &str = "(doc-page (dir prelude))";
    const LISTING_HEADER: &str = "(dir prelude)";
    const TYPE_SEPARATOR: &str = "\t";

    fn exported_names(&self) -> Vec<String> {
        let printed = self.evaluate(Self::LISTING_FORM);
        let listing = printed.trim().trim_matches('"').replace("\\n", "\n");
        listing
            .trim_start_matches(Self::LISTING_HEADER)
            .split_whitespace()
            .map(String::from)
            .collect()
    }

    fn inferred_types(&self, names: &[String]) -> Vec<(String, FunctionShape)> {
        let printing: String = names
            .iter()
            .map(|name| {
                format!(
                    "(println \"{name}{}\" (infer {name}))",
                    Self::TYPE_SEPARATOR
                )
            })
            .collect();
        self.evaluate(&format!("(do {printing})"))
            .lines()
            .filter_map(|line| line.split_once(Self::TYPE_SEPARATOR))
            .map(|(name, type_text)| (name.to_string(), FunctionShape::of(type_text)))
            .collect()
    }

    fn evaluate(&self, forms: &str) -> String {
        let mut child = Command::new(Self::PATH)
            .args([
                "eval",
                "--stdin",
                "--color",
                "never",
                Self::MODULE_FILE,
                forms,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the compiler starts");
        child
            .stdin
            .take()
            .expect("the compiler reads standard input")
            .write_all(Self::EMPTY_MODULE.as_bytes())
            .expect("the compiler takes the module text");
        let output = child.wait_with_output().expect("the compiler finishes");
        assert!(
            output.status.success(),
            "evaluating {forms} fails:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("the compiler prints UTF-8")
    }
}

#[test]
fn each_exported_predicate_and_mutation_carries_its_suffix() {
    let broken = PreludeNaming::load().broken_names();
    assert!(
        broken.is_empty(),
        "names that break the naming rules, outside the exceptions {:?}:\n{}",
        NamingException::ALL
            .iter()
            .map(|exception| (exception.name, exception.reason))
            .collect::<Vec<_>>(),
        broken.join("\n")
    );
}
