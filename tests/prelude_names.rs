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
    is_function: bool,
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
        match FormText::headed(form) {
            Some((Self::WHERE, [_, body])) => Self::of_form(body),
            Some((Self::FUNCTION, [parameters, result])) => Self {
                is_function: true,
                returns_bool: FormText::symbol(result) == Some(Self::BOOL),
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
            .any(|parameter| matches!(FormText::headed(parameter), Some((Self::MUT, _))))
    }
}

struct FormText;

impl FormText {
    const FILE: &str = "<example>";

    fn read(text: &str) -> Vec<Form> {
        let mut sources = SourceMap::default();
        let source = sources.add(Self::FILE, text);
        Reader::new(sources.file(source)).read().unwrap_or_default()
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

struct PreludeExamples {
    compiler: Compiler,
}

impl PreludeExamples {
    fn load() -> Self {
        Self { compiler: Compiler }
    }

    fn broken_examples(&self) -> Vec<String> {
        let names = self.compiler.exported_names();
        let rules = ExampleRules::of(self.functions(&names));
        self.compiler
            .pages(&names)
            .iter()
            .flat_map(|page| page.broken_examples(&rules))
            .collect()
    }

    fn functions(&self, names: &[String]) -> BTreeSet<String> {
        let values = ValueProbe::of(names.to_vec()).values();
        self.compiler
            .inferred_types(&values)
            .into_iter()
            .filter(|(_, shape)| shape.is_function)
            .map(|(name, _)| name)
            .collect()
    }
}

struct Page {
    name: String,
    body: String,
}

impl Page {
    fn broken_examples(&self, rules: &ExampleRules) -> Vec<String> {
        ExampleBlocks::of(&self.body)
            .iter()
            .enumerate()
            .flat_map(|(index, example)| {
                let number = index + 1;
                rules
                    .problems(&self.name, example)
                    .into_iter()
                    .map(move |problem| format!("{} #{number}: {problem}", self.name))
            })
            .collect()
    }
}

#[derive(Default)]
struct ExampleBlocks<'a> {
    examples: Vec<String>,
    paragraph: Vec<&'a str>,
    fence: Option<Fence<'a>>,
}

struct Fence<'a> {
    holds_code: bool,
    lines: Vec<&'a str>,
}

impl<'a> ExampleBlocks<'a> {
    const FENCE: &'static str = "```";
    const CODE_TAG: &'static str = "knave";
    const RESULT: &'static str = "=>";

    fn of(body: &'a str) -> Vec<String> {
        let mut blocks = Self::default();
        for line in body.lines() {
            blocks.take(line);
        }
        blocks.end_paragraph();
        blocks.examples
    }

    fn take(&mut self, line: &'a str) {
        if let Some(tag) = line.trim_start().strip_prefix(Self::FENCE) {
            self.toggle_fence(tag);
        } else if let Some(fence) = &mut self.fence {
            fence.lines.push(line);
        } else if line.trim().is_empty() {
            self.end_paragraph();
        } else {
            self.paragraph.push(line);
        }
    }

    fn toggle_fence(&mut self, tag: &str) {
        match self.fence.take() {
            Some(fence) if fence.holds_code => self.examples.push(Self::code(&fence.lines)),
            Some(_) => {}
            None => {
                self.end_paragraph();
                self.fence = Some(Fence {
                    holds_code: tag.is_empty() || tag == Self::CODE_TAG,
                    lines: Vec::new(),
                });
            }
        }
    }

    fn end_paragraph(&mut self) {
        let lines = std::mem::take(&mut self.paragraph);
        if lines.iter().any(|line| line.starts_with(Self::RESULT)) {
            self.examples.push(Self::code(&lines));
        }
    }

    fn code(lines: &[&str]) -> String {
        lines
            .iter()
            .filter(|line| !line.starts_with(Self::RESULT))
            .map(|line| format!("{line}\n"))
            .collect()
    }
}

struct ExampleRules {
    functions: BTreeSet<String>,
}

impl ExampleRules {
    const UNWRAPS: &[&str] = &["unwrap", "expect", "unwrap-ok", "unwrap-err", "expect-ok"];
    const CONSTRUCTORS: &[&str] = &[
        "str",
        "vec",
        "hash-map",
        "sorted-map",
        "some",
        "ok",
        "err",
        "pair",
        "box",
        "char",
        "seconds",
        "millis",
        "micros",
        "nanos",
    ];
    const TEST: &str = "deftest";
    const THREADS: &[&str] = &["->", "->>"];
    const FIRST_STEP: usize = 2;

    fn of(functions: BTreeSet<String>) -> Self {
        Self { functions }
    }

    fn problems(&self, page: &str, example: &str) -> Vec<String> {
        let mut problems = Vec::new();
        for form in FormText::read(example) {
            self.walk(page, &form, &mut problems);
        }
        problems
    }

    fn walk(&self, page: &str, form: &Form, problems: &mut Vec<String>) {
        let Some(items) = form.as_list() else {
            return;
        };
        let head = items.first().and_then(FormText::symbol);
        if head == Some(Self::TEST) {
            return;
        }
        problems.extend(head.and_then(|head| self.problem(page, head, items)));
        let steps = if head.is_some_and(|head| Self::THREADS.contains(&head)) {
            Self::FIRST_STEP
        } else {
            items.len()
        };
        for (position, item) in items.iter().enumerate() {
            if position < steps {
                self.walk(page, item, problems);
            } else {
                self.walk_step(page, item, problems);
            }
        }
    }

    fn walk_step(&self, page: &str, step: &Form, problems: &mut Vec<String>) {
        for item in step.as_list().unwrap_or_default() {
            self.walk(page, item, problems);
        }
    }

    fn problem(&self, page: &str, head: &str, items: &[Form]) -> Option<String> {
        if Self::UNWRAPS.contains(&head) && head != page {
            return Some(format!("calls `{head}` outside its own page"));
        }
        let first = items.get(1).and_then(FormText::headed)?.0;
        (self.starts_chain(head) && self.starts_chain(first))
            .then(|| format!("`({head} ({first} ...))` is not threaded with `->` or `->>`"))
    }

    fn starts_chain(&self, name: &str) -> bool {
        self.functions.contains(name) && !Self::CONSTRUCTORS.contains(&name)
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
    const PAGE_MARKER: &str = "@@page ";

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

    fn pages(&self, names: &[String]) -> Vec<Page> {
        let printing: String = names
            .iter()
            .map(|name| {
                format!(
                    "(println \"{}{name}\") (println (doc-body {name}))",
                    Self::PAGE_MARKER
                )
            })
            .collect();
        self.evaluate(&format!("(do {printing})"))
            .split(Self::PAGE_MARKER)
            .skip(1)
            .filter_map(|chunk| chunk.split_once('\n'))
            .map(|(name, body)| Page {
                name: name.to_string(),
                body: body.to_string(),
            })
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

#[test]
fn each_prelude_example_follows_the_code_rules_of_the_skill() {
    let broken = PreludeExamples::load().broken_examples();
    assert!(
        broken.is_empty(),
        "prelude examples that break the code rules of `skills/writing-knave/SKILL.md`:\n{}",
        broken.join("\n")
    );
}

struct RulesFixture;

impl RulesFixture {
    const FUNCTIONS: &[&str] = &["collect", "map", "iter", "some", "unwrap-ok"];
    const PAGE: &str = "ok";

    fn problems(example: &str) -> Vec<String> {
        Self::rules().problems(Self::PAGE, example)
    }

    fn problems_on(page: &str, example: &str) -> Vec<String> {
        Self::rules().problems(page, example)
    }

    fn rules() -> ExampleRules {
        ExampleRules::of(
            Self::FUNCTIONS
                .iter()
                .map(|name| name.to_string())
                .collect(),
        )
    }
}

#[test]
fn an_unwrap_outside_its_own_page_breaks_the_rules() {
    assert_eq!(
        RulesFixture::problems("(unwrap-ok (ok 1))"),
        ["calls `unwrap-ok` outside its own page"]
    );
    assert!(RulesFixture::problems_on("unwrap-ok", "(unwrap-ok (ok 1))").is_empty());
}

#[test]
fn an_unwrap_inside_a_test_breaks_no_rule() {
    assert!(RulesFixture::problems("(deftest reads (unwrap-ok (ok 1)))").is_empty());
}

#[test]
fn a_call_on_a_call_breaks_the_rules_unless_threaded() {
    assert_eq!(
        RulesFixture::problems("(collect (map items str))"),
        ["`(collect (map ...))` is not threaded with `->` or `->>`"]
    );
    assert!(RulesFixture::problems("(-> items (map str) collect)").is_empty());
    assert!(RulesFixture::problems("(-> items (map (iter other) str))").is_empty());
    assert_eq!(
        RulesFixture::problems("(-> (collect (map items str)) iter)").len(),
        1
    );
}

#[test]
fn a_constructor_or_another_form_starts_no_chain() {
    assert!(RulesFixture::problems("(some (map items str))").is_empty());
    assert!(RulesFixture::problems("(collect (some 1))").is_empty());
    assert!(RulesFixture::problems("(the int (map items str))").is_empty());
    assert!(RulesFixture::problems("(collect (the int items))").is_empty());
}

#[test]
fn examples_are_result_paragraphs_and_code_fences() {
    let body = "Prose.\n\n(f 1)\n=> 2\n\n(g 1)\n\n```\n(h 1)\n```\n\n```knave\n(k 1)\n=> 3\n```\n\n```text\n(t 1)\n```";
    assert_eq!(ExampleBlocks::of(body), ["(f 1)\n", "(h 1)\n", "(k 1)\n"]);
}

#[test]
fn only_a_function_type_is_a_function() {
    assert!(FunctionShape::of("(fn (int) int)").is_function);
    assert!(FunctionShape::of("(where (a) (fn (a) a))").is_function);
    assert!(!FunctionShape::of("int").is_function);
}

#[test]
fn only_a_prelude_function_joins_the_functions_of_the_rules() {
    let names = ["map", "->"].map(String::from);
    let functions = PreludeExamples::load().functions(&names);
    assert_eq!(functions, BTreeSet::from(["map".to_string()]));
}

#[test]
fn a_last_paragraph_and_a_paragraph_before_a_fence_are_examples() {
    let body = "(f 1)\n=> 2\n```\n(h 1)\n```\n(g 1)\n=> 3";
    assert_eq!(ExampleBlocks::of(body), ["(f 1)\n", "(h 1)\n", "(g 1)\n"]);
}

#[test]
fn an_unwrap_inside_a_thread_step_breaks_the_rules() {
    assert_eq!(
        RulesFixture::problems("(-> items (map (unwrap-ok item)))"),
        ["calls `unwrap-ok` outside its own page"]
    );
}

#[test]
fn only_a_listed_name_is_excused() {
    assert!(NamingException::excuses("set!"));
    assert!(!NamingException::excuses("empty?"));
}

#[test]
fn an_enum_case_names_its_enum_before_the_slash() {
    assert!(PreludeNaming::is_enum_case("option/some"));
    assert!(!PreludeNaming::is_enum_case("/"));
    assert!(!PreludeNaming::is_enum_case("map"));
}

#[test]
fn the_checked_names_leave_out_enum_cases() {
    let names = PreludeNaming::load().exported_names();
    assert!(names.iter().any(|name| name == "map"));
    assert!(!names.iter().any(|name| PreludeNaming::is_enum_case(name)));
}
