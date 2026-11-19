use std::collections::BTreeSet;
use std::env;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use knave_agree::LeftOutFixtures;
use rustc_hash::FxHashMap;

use super::fixture_queue::FixtureQueue;

static ANNOUNCED_NOTES: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

pub struct FixtureReports {
    reports: FxHashMap<PathBuf, String>,
    left_out: Option<LeftOutFixtures>,
}

impl FixtureReports {
    const JOB_LIMIT: &str = "CARGO_BUILD_JOBS";
    const FIXTURE_DEADLINE: Duration = Duration::from_secs(300);

    pub fn compute(
        paths: Vec<PathBuf>,
        report: impl Fn(&Path) -> String + Send + Sync + 'static,
    ) -> Self {
        let left_out = LeftOutFixtures::from_environment();
        Self::announce_once(&left_out.notes_for(&paths));
        Self::compute_within(Self::FIXTURE_DEADLINE, left_out.kept(paths), report)
            .leaving_out(left_out)
    }

    pub fn compute_within(
        deadline: Duration,
        paths: Vec<PathBuf>,
        report: impl Fn(&Path) -> String + Send + Sync + 'static,
    ) -> Self {
        let queue = Arc::new(FixtureQueue::new(paths));
        let report = Arc::new(report);
        for _ in 0..Self::workers() {
            let queue = Arc::clone(&queue);
            let report = Arc::clone(&report);
            thread::spawn(move || queue.drain(&*report));
        }
        Self {
            reports: queue.finished_reports(deadline),
            left_out: None,
        }
    }

    pub fn leaving_out(self, left_out: LeftOutFixtures) -> Self {
        Self {
            left_out: Some(left_out),
            ..self
        }
    }

    pub fn report(&self, path: &Path) -> Option<&str> {
        let report = self.reports.get(path).map(String::as_str);
        assert!(
            report.is_some() || self.leaves_out(path),
            "no report for {}: the first glob pass listed it, and no left-out list names it",
            path.display()
        );
        report
    }

    fn leaves_out(&self, path: &Path) -> bool {
        self.left_out
            .as_ref()
            .is_some_and(|left_out| left_out.leaves_out(path))
    }

    fn announce_once(notes: &[String]) {
        let mut announced = ANNOUNCED_NOTES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut stderr = io::stderr();
        for note in notes {
            if announced.insert(note.clone()) {
                let _shown_only_when_stderr_is_open = writeln!(stderr, "{note}");
            }
        }
    }

    fn workers() -> usize {
        env::var(Self::JOB_LIMIT)
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or_else(|| thread::available_parallelism().map_or(1, NonZeroUsize::get))
    }
}
