use std::env;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
        Self::compute_within(Self::FIXTURE_DEADLINE, paths, report)
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

    pub fn report(&self, path: &Path) -> &str {
        self.reports
            .get(path)
            .expect("the first glob pass listed this fixture")
    }

    fn workers() -> usize {
        env::var(Self::JOB_LIMIT)
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or_else(|| thread::available_parallelism().map_or(1, NonZeroUsize::get))
    }
}
