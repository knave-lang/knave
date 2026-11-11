use std::path::PathBuf;

use super::Expander;
use crate::core::ModulePath;
use crate::diagnostic::DiagnosticCode;
use crate::modules::ModuleSources;
use crate::source::SourceMap;

struct MacroRun;

impl MacroRun {
    const ENTRY: &str = "work";
    const RUN: &str = "(def x (run))";
    const BUILTINS: &str = "(use code add-module-note concat escape-char escape-string first list \
                            located number-sub prepend read-form rest string string-lines string-lowercase \
                            symbol value-eq)";

    fn too_large(body: &str) -> Option<String> {
        Self::failure_texts(body, &DiagnosticCode::MACRO_VALUE_SIZE)
            .into_iter()
            .next()
    }

    fn runs_out_of_steps(body: &str, steps: u64) -> bool {
        let run = format!("(with-macro-steps {steps} (def x (run)))");
        !Self::failures_of(body, &run, &DiagnosticCode::MACRO_STEP_LIMIT).is_empty()
    }

    fn failure_texts(body: &str, code: &DiagnosticCode) -> Vec<String> {
        Self::failures_of(body, Self::RUN, code)
    }

    fn failures_of(body: &str, run: &str, code: &DiagnosticCode) -> Vec<String> {
        let mut sources = SourceMap::default();
        let program = format!("{}\n{body}\n{run}\n", Self::BUILTINS);
        let entry = sources.add(Self::ENTRY, program);
        let mut expander = Expander::new(
            &mut sources,
            ModuleSources::new(PathBuf::new(), PathBuf::new()),
        );
        expander.load_source_module(ModulePath::new(Self::ENTRY), &[entry]);
        let spans: Vec<_> = expander
            .take_diagnostics_with_code(code)
            .iter()
            .map(|diagnostic| diagnostic.primary().span())
            .collect();
        drop(expander);
        spans
            .into_iter()
            .map(|span| sources.file(span.source()).text()[span.range()].to_string())
            .collect()
    }

    fn growing(start: &str, growth: &str) -> String {
        format!(
            "(def grow (fn (x) (recur {growth})))
(def run (macro (form) (grow {start})))"
        )
    }

    fn doubled_text(text: &str, doublings: usize) -> String {
        format!(
            "(def double (fn (x k) (if (value-eq k 0) x (recur (string x x) (number-sub k 1)))))
(def text (double \"{text}\" {doublings}))"
        )
    }
}

#[test]
fn joining_text_stops_at_the_text_limit() {
    let growth = "(string x x)";
    let stopped = MacroRun::too_large(&MacroRun::growing("\"x\"", growth));
    assert_eq!(stopped.as_deref(), Some(growth));
}

#[test]
fn joining_a_symbol_name_stops_at_the_text_limit() {
    let growth = "(symbol x x x)";
    let stopped = MacroRun::too_large(&MacroRun::growing("(quote x)", growth));
    assert_eq!(stopped.as_deref(), Some(growth));
}

#[test]
fn escaping_text_stops_at_the_text_limit() {
    for growth in ["(escape-string x)", "(escape-char x)"] {
        let stopped = MacroRun::too_large(&MacroRun::growing("\"\\\\\"", growth));
        assert_eq!(stopped.as_deref(), Some(growth));
    }
}

#[test]
fn appending_a_list_to_itself_stops_at_the_list_limit() {
    let growth = "(concat x x)";
    let stopped = MacroRun::too_large(&MacroRun::growing("(list 1)", growth));
    assert_eq!(stopped.as_deref(), Some(growth));
}

#[test]
fn sharing_a_list_twice_counts_it_twice() {
    let growth = "(list x x)";
    let stopped = MacroRun::too_large(&MacroRun::growing("(list 1)", growth));
    assert_eq!(stopped.as_deref(), Some(growth));
}

#[test]
fn prepending_a_list_onto_itself_stops_at_the_list_limit() {
    let growth = "(prepend x x)";
    let stopped = MacroRun::too_large(&MacroRun::growing("(list 1)", growth));
    assert_eq!(stopped.as_deref(), Some(growth));
}

#[test]
fn sharing_the_rest_of_a_list_twice_counts_it_twice() {
    let growth = "(list (rest x) (rest x))";
    let stopped = MacroRun::too_large(&MacroRun::growing("(list 1 2)", growth));
    assert_eq!(stopped.as_deref(), Some(growth));
}

#[test]
fn a_list_counts_a_form_by_its_tree() {
    let body = format!(
        "{}
(def tree (first (read-form (quote x) (string \"(\" text \")\") 0)))
(def run (macro (form) (list tree tree tree tree tree tree tree tree tree tree tree tree tree tree tree tree tree)))",
        MacroRun::doubled_text("a ", 18)
    );
    let stopped = MacroRun::too_large(&body);
    assert!(stopped.is_some_and(|call| call.starts_with("(list tree")));
}

#[test]
fn joining_stops_at_the_first_part_past_the_text_limit() {
    let body = format!(
        "{}
(def run (macro (form) (string text text first)))",
        MacroRun::doubled_text("x", 24)
    );
    assert_eq!(
        MacroRun::too_large(&body).as_deref(),
        Some("(string text text first)")
    );
}

#[test]
fn joining_a_shared_list_stops_writing_at_the_text_limit() {
    let body = format!(
        "{}
(def share (fn (x k) (if (value-eq k 0) x (recur (list x x) (number-sub k 1)))))
(def run (macro (form) (string (share text 16))))",
        MacroRun::doubled_text("x", 10)
    );
    assert_eq!(
        MacroRun::too_large(&body).as_deref(),
        Some("(string (share text 16))")
    );
}

#[test]
fn lowercasing_text_that_grows_stops_at_the_text_limit() {
    let body = format!(
        "{}
(def run (macro (form) (do (string-lowercase text) (quote 1))))",
        MacroRun::doubled_text("\u{130}", 23)
    );
    assert_eq!(
        MacroRun::too_large(&body).as_deref(),
        Some("(string-lowercase text)")
    );
}

#[test]
fn reading_a_form_past_the_list_limit_stops_at_it() {
    let body = format!(
        "{}
(def run (macro (form) (do (read-form (quote x) (string \"(\" text \")\") 0) (quote 1))))",
        MacroRun::doubled_text("a ", 22)
    );
    assert_eq!(
        MacroRun::too_large(&body).as_deref(),
        Some("(read-form (quote x) (string \"(\" text \")\") 0)")
    );
}

#[test]
fn reading_forms_spends_steps_for_each_form_it_builds() {
    let body = format!(
        "{}
(def run (macro (form) (do (read-form (quote x) (string \"(\" text \")\") 0) (quote 1))))",
        MacroRun::doubled_text("a ", 10)
    );
    assert!(MacroRun::runs_out_of_steps(&body, 2000));
    assert!(!MacroRun::runs_out_of_steps(&body, 20000));
}

#[test]
fn adding_a_module_note_spends_steps_for_the_bytes_it_holds() {
    let body = "(def share (fn (x k) (if (value-eq k 0) x (recur (list x x) (number-sub k 1)))))
(def tree (share (quote a) 10))
(def run (macro (form) (do (add-module-note (quote k) tree) (quote 1))))";
    assert!(MacroRun::runs_out_of_steps(body, 3000));
    assert!(!MacroRun::runs_out_of_steps(body, 20000));
}

#[test]
fn splitting_text_into_too_many_lines_stops_at_the_list_limit() {
    let body = format!(
        "{}
(def run (macro (form) (string-lines text)))",
        MacroRun::doubled_text("\\n", 23)
    );
    assert_eq!(
        MacroRun::too_large(&body).as_deref(),
        Some("(string-lines text)")
    );
}

#[test]
fn adding_module_notes_past_their_byte_limit_stops_at_it() {
    let body = format!(
        "{}
(def add (fn (n) (if (value-eq n 0) (quote 1) (do (add-module-note (quote k) text) (recur (number-sub n 1))))))
(def run (macro (form) (add 20)))",
        MacroRun::doubled_text("x", 24)
    );
    assert_eq!(
        MacroRun::too_large(&body).as_deref(),
        Some("(add-module-note (quote k) text)")
    );
}

#[test]
fn a_value_within_every_limit_builds() {
    let body = format!(
        "{}
(def run (macro (form) (do (list text text) (string-lines text) (quote 1))))",
        MacroRun::doubled_text("ab", 10)
    );
    assert_eq!(MacroRun::too_large(&body), None);
}

#[test]
fn a_macro_used_as_a_value_reports_the_value_it_built_too_large() {
    let body = "(def big (macro (form) ((fn (x) (recur (string x x))) \"x\")))
(def use-it (fn () big))
(def run (macro (form) (quote 1)))";
    assert_eq!(MacroRun::too_large(body).as_deref(), Some("(string x x)"));
}
