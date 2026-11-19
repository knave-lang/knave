use std::env;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use knave_agree::LeftOutFixtures;
use rustc_hash::FxHashMap;

use super::fixture_queue::FixtureQueue;

pub struct FixtureReports {
    reports: FxHashMap<PathBuf, String>,
}

impl FixtureReports {
    const JOB_LIMIT: &str = "CARGO_BUILD_JOBS";
    const FIXTURE_DEADLINE: Duration = Duration::from_secs(300);

    pub fn compute(
        paths: Vec<PathBuf>,
        report: impl Fn(&Path) -> String + Send + Sync + 'static,
    ) -> Self {
        let left_out = LeftOutFixtures::from_environment();
        Self::announce(&left_out.notes_for(&paths));
        Self::compute_within(Self::FIXTURE_DEADLINE, left_out.kept(paths), report)
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
        }
    }

    pub fn report(&self, path: &Path) -> Option<&str> {
        self.reports.get(path).map(String::as_str)
    }

    fn announce(notes: &[String]) {
        let mut stderr = io::stderr();
        for note in notes {
            let _shown_only_when_stderr_is_open = writeln!(stderr, "{note}");
        }
    }

    fn workers() -> usize {
        env::var(Self::JOB_LIMIT)
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or_else(|| thread::available_parallelism().map_or(1, NonZeroUsize::get))
    }
}
