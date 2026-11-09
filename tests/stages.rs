mod support;

use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use support::{Fixture, FixtureReports, FormatFailures, HeaderAgreement, SavedPreludeAgreement};

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
