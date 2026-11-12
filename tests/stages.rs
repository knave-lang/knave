mod support;

use std::collections::BTreeSet;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use knave::core::KernelForm;
use knave::stack::FreshStack;

use support::{
    AgreementReport, ExplanationExamples, Fixture, FixtureReports, FormatFailures, HeaderAgreement,
    LintFixture, ReadBackAgreement, SavedPreludeAgreement,
};

const BORROW_ACROSS_MOVE_FIX: &str = "## Fixes\n\n```lisp\n(defn main ()\n  (local grid (vec 1 2))\n  (local first (at grid 0))\n  (local next (assoc grid 0 3))\n  (println (at next 0) \" \" first))\n```\n";
const SECOND_TRAIT_PATH_EXAMPLE: &str = "## Erroneous example\n\n```lisp\n(deftrait printable (rust \"std::fmt::Display\") i64)\n\n(defn main ()\n  (println 1))\n```\n";
const TITLE_QUOTING_EXPLANATION: &str = "The compiler says cannot find `distnce` in this scope.\n\n## Erroneous example\n\n```lisp\n(defn main ()\n  (println distnce))\n```\n";
const WARNING_TITLE_QUOTING_EXPLANATION: &str = "The compiler says `printable` binds a Rust path that `display` already binds.\n\n## Erroneous example\n\n```lisp\n(deftrait printable (rust \"std::fmt::Display\") i64)\n\n(defn main ()\n  (println 1))\n```\n";
const MISSING_MAIN_EXAMPLE: &str = "## Erroneous example\n\n```lisp\n(defn helper () 1)\n```\n";
const PROJECT_EXAMPLE: &str = "## Erroneous example\n\n```lisp\n;; project.knv\n(libraries)\n;; src/main.knv\n(defn main () (println 1))\n```\n";
const REPL_EXAMPLE: &str = "## Erroneous example\n\n```lisp\n;; knave repl\n(defstruct point :x i64)\n(defn ->point (x) x)\n```\n";
const COMMENTED_EXAMPLE: &str = "## Erroneous example\n\n```lisp\n;; knave keeps this note\n;; total.knv holds the sum\n(defn main () (println totl))\n```\n";
const FAILING_EXIT_FIX: &str = "## Fixes\n\n```lisp\n(defn main ()\n  (local empty (the (vec i64) (vec)))\n  (println (at empty 0)))\n```\n";

#[test]
fn reader_fixtures() {
    let fixtures = "fixtures/read/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).read_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn project_fixtures() {
    let fixtures = "fixtures/project/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).project_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn format_fixtures() {
    let fixtures = "fixtures/format/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).format_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn every_repository_source_formats_to_the_same_forms_and_stays_formatted() {
    let failures = FormatFailures::of_repository();
    eprintln!(
        "formatted {} files in {:?}",
        failures.formatted, failures.elapsed
    );
    assert!(
        failures.failed.is_empty(),
        "the formatter fails its checks on:\n{}",
        failures.failed.join("\n")
    );
}

#[test]
fn expand_fixtures() {
    let fixtures = "fixtures/expand/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).expand_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn check_fixtures() {
    let fixtures = "fixtures/check/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).check_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn emit_fixtures() {
    let fixtures = "fixtures/emit/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).emit_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn run_fixtures() {
    let fixtures = "fixtures/run/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).run_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn lint_fixtures() {
    let fixtures = "fixtures/lint/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| LintFixture::load(path).report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn lint_reports_an_unread_binding_in_the_deepest_definition() {
    let body = (0..GeneratedLint::DEEPEST_NESTING)
        .fold(String::from("x"), |inner, _| format!("(if true {inner} 0)"));
    let text = format!(
        "(defn deep (x)\n  (let (unused 1)\n    {body}))\n\n(defn main () (println (deep 1)))\n"
    );
    let report = GeneratedLint::report("deepest_definition.knv", text);
    assert!(
        report.contains("warning[L0801]: `unused` is never read"),
        "{report}"
    );
}

#[test]
fn the_lint_read_back_prints_each_definition_as_knave_expand_does() {
    let disagreements = ReadBackAgreement::load().disagreements();
    assert!(
        disagreements.is_empty(),
        "the read-back and the printer differ outside {:?}:\n{}",
        ReadBackAgreement::DELIBERATE_DIFFERENCES,
        disagreements.join("\n")
    );
}

#[test]
fn the_read_back_agreement_holds_every_kernel_form_a_definition_can_hold() {
    let held = ReadBackAgreement::load().kernel_heads();
    let expected: BTreeSet<String> = KernelForm::all()
        .map(|form| form.name().to_string())
        .filter(|name| !ReadBackAgreement::is_never_held(name))
        .collect();
    assert_eq!(
        held,
        expected,
        "forms no read-back holds: {:?}",
        ReadBackAgreement::NEVER_HELD
    );
}

#[test]
fn corpus_fixtures() {
    let fixtures = "fixtures/corpus/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let reports = FixtureReports::compute(paths, |path| Fixture::load(path).run_report());
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn agree_fixtures() {
    let fixtures = "fixtures/agree/*.knv";
    let mut paths = Vec::new();
    insta::glob!(fixtures, |path| paths.push(PathBuf::from(path)));
    let agreement = AgreementReport::new();
    let reports = FixtureReports::compute(paths, move |path| agreement.of(path));
    insta::glob!(fixtures, |path| {
        insta::assert_snapshot!(reports.report(path));
    });
}

#[test]
fn defn_headers_agree() {
    HeaderAgreement::of("defn").assert_headers_agree();
}

#[test]
fn defmacro_headers_agree() {
    HeaderAgreement::of("defmacro").assert_headers_agree();
}

#[test]
fn defstruct_headers_agree() {
    HeaderAgreement::of("defstruct").assert_headers_agree();
}

#[test]
fn defrule_headers_agree() {
    HeaderAgreement::of("defrule").assert_headers_agree();
}

#[test]
fn saved_prelude_expand_fixtures() {
    let mut paths = Vec::new();
    insta::glob!("fixtures/expand/*.knv", |path| paths
        .push(PathBuf::from(path)));
    SavedPreludeAgreement::of(paths).assert_loaded_prelude_agrees();
}

#[test]
fn saved_prelude_check_fixtures() {
    let mut paths = Vec::new();
    insta::glob!("fixtures/check/*.knv", |path| paths
        .push(PathBuf::from(path)));
    SavedPreludeAgreement::of(paths).assert_loaded_prelude_agrees();
}

#[test]
fn saved_prelude_emit_fixtures() {
    let mut paths = Vec::new();
    insta::glob!("fixtures/emit/*.knv", |path| paths
        .push(PathBuf::from(path)));
    SavedPreludeAgreement::of(paths).assert_loaded_prelude_agrees();
}

#[test]
fn saved_prelude_run_fixtures() {
    let mut paths = Vec::new();
    insta::glob!("fixtures/run/*.knv", |path| paths.push(PathBuf::from(path)));
    SavedPreludeAgreement::of(paths).assert_loaded_prelude_agrees();
}

#[test]
fn saved_prelude_corpus_fixtures() {
    let mut paths = Vec::new();
    insta::glob!("fixtures/corpus/*.knv", |path| paths
        .push(PathBuf::from(path)));
    SavedPreludeAgreement::of(paths).assert_loaded_prelude_agrees();
}

const SHORT_DEADLINE: Duration = Duration::from_millis(50);
const SLOW_FIXTURE: Duration = Duration::from_millis(500);
const MORE_FIXTURES_THAN_WORKERS: usize = 1024;

#[test]
#[should_panic(expected = "the fixture slow.knv ran longer than")]
fn a_fixture_past_its_deadline_fails_naming_the_fixture() {
    FixtureReports::compute_within(SHORT_DEADLINE, vec![PathBuf::from("slow.knv")], |_| {
        thread::sleep(SHORT_DEADLINE * 10);
        String::new()
    });
}

#[test]
fn a_fixture_past_its_deadline_stops_the_fixtures_still_waiting() {
    let started = Arc::new(AtomicUsize::new(0));
    let starts = Arc::clone(&started);
    let paths = (0..MORE_FIXTURES_THAN_WORKERS)
        .map(|index| PathBuf::from(format!("waiting{index}.knv")))
        .collect();
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        FixtureReports::compute_within(SHORT_DEADLINE, paths, move |_| {
            starts.fetch_add(1, Ordering::SeqCst);
            thread::sleep(SLOW_FIXTURE);
            String::new()
        })
    }));
    assert!(outcome.is_err(), "the slow fixtures pass their deadline");
    let started_by_the_deadline = started.load(Ordering::SeqCst);
    thread::sleep(SLOW_FIXTURE * 3);
    assert_eq!(started.load(Ordering::SeqCst), started_by_the_deadline);
}

#[test]
fn a_fixture_that_panics_stops_the_fixtures_still_waiting() {
    let started = Arc::new(AtomicUsize::new(0));
    let starts = Arc::clone(&started);
    let paths = (0..MORE_FIXTURES_THAN_WORKERS)
        .map(|index| PathBuf::from(format!("waiting{index}.knv")))
        .collect();
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        FixtureReports::compute_within(SLOW_FIXTURE * 100, paths, move |_| {
            starts.fetch_add(1, Ordering::SeqCst);
            thread::sleep(SLOW_FIXTURE);
            panic!("each report fails")
        })
    }));
    assert!(outcome.is_err(), "the reports panic");
    let started_by_the_panic = started.load(Ordering::SeqCst);
    thread::sleep(SLOW_FIXTURE * 3);
    assert_eq!(started.load(Ordering::SeqCst), started_by_the_panic);
}

#[test]
#[should_panic(expected = "the report of broken.knv fails")]
fn a_fixture_whose_report_panics_fails_with_its_panic() {
    FixtureReports::compute_within(SHORT_DEADLINE, vec![PathBuf::from("broken.knv")], |path| {
        panic!("the report of {} fails", path.display())
    });
}

#[test]
fn fixtures_within_their_deadline_keep_their_reports() {
    let path = PathBuf::from("quick.knv");
    let reports = FixtureReports::compute_within(SHORT_DEADLINE, vec![path.clone()], |path| {
        path.display().to_string()
    });
    assert_eq!(reports.report(&path), "quick.knv");
}

#[test]
fn each_erroneous_explanation_example_reports_its_code_first() {
    for explanation in ExplanationExamples::written() {
        let first_codes = explanation.erroneous_first_codes();
        assert!(
            !first_codes.is_empty(),
            "{} has no erroneous example",
            explanation.code()
        );
        for first in first_codes {
            assert_eq!(first.as_deref(), Some(explanation.code()));
        }
    }
}

#[test]
fn each_explanation_fix_builds_and_runs() {
    for explanation in ExplanationExamples::written() {
        assert!(
            explanation.fix_count() > 0,
            "{} has no fix",
            explanation.code()
        );
        assert_eq!(explanation.fix_failures(), Vec::<String>::new());
    }
}

#[test]
fn an_explanation_example_that_only_warns_reports_its_warning_code() {
    let explanation = ExplanationExamples::new("L0217", SECOND_TRAIT_PATH_EXAMPLE);
    assert_eq!(
        explanation.erroneous_first_codes(),
        vec![Some("L0217".to_string())]
    );
}

#[test]
fn an_explanation_example_that_only_emit_rejects_reports_the_emit_code() {
    let explanation = ExplanationExamples::new("L0601", MISSING_MAIN_EXAMPLE);
    assert_eq!(
        explanation.erroneous_first_codes(),
        vec![Some("L0601".to_string())]
    );
}

#[test]
fn an_explanation_example_labeled_with_files_runs_as_a_project() {
    let explanation = ExplanationExamples::new("L0701", PROJECT_EXAMPLE);
    assert_eq!(
        explanation.erroneous_first_codes(),
        vec![Some("L0701".to_string())]
    );
}

#[test]
fn an_explanation_example_labeled_with_a_command_runs_it() {
    let explanation = ExplanationExamples::new("L0249", REPL_EXAMPLE);
    assert_eq!(
        explanation.erroneous_first_codes(),
        vec![Some("L0249".to_string())]
    );
}

#[test]
fn an_explanation_example_with_ordinary_comments_reads_as_one_file() {
    let explanation = ExplanationExamples::new("L0201", COMMENTED_EXAMPLE);
    assert_eq!(
        explanation.erroneous_first_codes(),
        vec![Some("L0201".to_string())]
    );
}

#[test]
fn an_explanation_fix_that_rustc_refuses_fails_its_run() {
    let explanation = ExplanationExamples::new("rustc-refusal", BORROW_ACROSS_MOVE_FIX);
    assert_eq!(explanation.fix_failures().len(), 1);
}

#[test]
fn an_explanation_fix_that_exits_nonzero_fails_its_run() {
    let explanation = ExplanationExamples::new("nonzero-exit", FAILING_EXIT_FIX);
    assert_eq!(explanation.fix_failures().len(), 1);
}

#[test]
fn an_explanation_leaves_the_diagnostic_wording_to_its_problems_type() {
    for explanation in ExplanationExamples::written() {
        assert_eq!(
            explanation.restated_titles(),
            Vec::<String>::new(),
            "{} restates its diagnostic",
            explanation.code()
        );
    }
}

#[test]
fn an_explanation_that_quotes_its_own_title_restates_it() {
    let explanation = ExplanationExamples::new("L0201", TITLE_QUOTING_EXPLANATION);
    assert_eq!(
        explanation.restated_titles(),
        vec!["cannot find `distnce` in this scope".to_string()]
    );
}

#[test]
fn an_explanation_that_quotes_its_warning_title_restates_it() {
    let explanation = ExplanationExamples::new("L0217", WARNING_TITLE_QUOTING_EXPLANATION);
    assert_eq!(
        explanation.restated_titles(),
        vec!["`printable` binds a Rust path that `display` already binds".to_string()]
    );
}

#[test]
fn an_explanation_may_quote_the_title_of_another_code() {
    let explanation = ExplanationExamples::new("L0501", TITLE_QUOTING_EXPLANATION);
    assert_eq!(explanation.restated_titles(), Vec::<String>::new());
}

struct GeneratedLint;

impl GeneratedLint {
    const DEEPEST_NESTING: usize = 1022;
    const STACK: FreshStack = FreshStack::of(32 * 1024 * 1024);

    fn report(name: &str, text: String) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lint")
            .join(name);
        Self::STACK.run(|| LintFixture::of_text(&path, text).report())
    }
}
