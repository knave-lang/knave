use std::env;
use std::ffi::{OsStr, OsString};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

use rustc_hash::FxHashMap;

pub struct FixtureReports {
    reports: FxHashMap<PathBuf, String>,
}

impl FixtureReports {
    const JOB_LIMIT: &str = "CARGO_BUILD_JOBS";
    const WORKER_LIMIT: &str = "KNAVE_FIXTURE_WORKERS";

    pub fn compute(paths: Vec<PathBuf>, report: impl Fn(&Path) -> String + Sync) -> Self {
        let pending = Mutex::new(paths);
        let finished = Mutex::new(FxHashMap::default());
        thread::scope(|scope| {
            for _ in 0..Self::workers() {
                scope.spawn(|| Self::drain(&pending, &finished, &report));
            }
        });
        Self {
            reports: finished.into_inner().expect("every worker finished"),
        }
    }

    pub fn report(&self, path: &Path) -> &str {
        self.reports
            .get(path)
            .expect("the first glob pass listed this fixture")
    }

    fn workers() -> usize {
        Self::workers_under(env::var_os(Self::WORKER_LIMIT))
    }

    fn workers_under(limit: Option<OsString>) -> usize {
        limit.map_or_else(Self::default_workers, |limit| Self::parsed_limit(&limit))
    }

    fn parsed_limit(limit: &OsStr) -> usize {
        limit
            .to_str()
            .and_then(|text| text.parse::<NonZeroUsize>().ok())
            .unwrap_or_else(|| {
                panic!(
                    "{} must be a positive whole number, not {limit:?}",
                    Self::WORKER_LIMIT
                )
            })
            .get()
    }

    fn default_workers() -> usize {
        env::var(Self::JOB_LIMIT)
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or_else(|| thread::available_parallelism().map_or(1, NonZeroUsize::get))
    }

    fn drain(
        pending: &Mutex<Vec<PathBuf>>,
        finished: &Mutex<FxHashMap<PathBuf, String>>,
        report: &impl Fn(&Path) -> String,
    ) {
        while let Some(path) = Self::next(pending) {
            let text = report(&path);
            finished
                .lock()
                .expect("no worker panics while holding the reports")
                .insert(path, text);
        }
    }

    fn next(pending: &Mutex<Vec<PathBuf>>) -> Option<PathBuf> {
        pending
            .lock()
            .expect("no worker panics while holding the queue")
            .pop()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::FixtureReports;

    #[test]
    fn an_unset_worker_limit_keeps_the_default_workers() {
        assert_eq!(
            FixtureReports::workers_under(None),
            FixtureReports::default_workers()
        );
    }

    #[test]
    fn a_worker_limit_sets_the_worker_count() {
        assert_eq!(FixtureReports::workers_under(Some(OsString::from("4"))), 4);
    }

    #[test]
    #[should_panic(expected = "KNAVE_FIXTURE_WORKERS must be a positive whole number")]
    fn a_worker_limit_of_zero_fails() {
        FixtureReports::workers_under(Some(OsString::from("0")));
    }

    #[test]
    #[should_panic(expected = "KNAVE_FIXTURE_WORKERS must be a positive whole number")]
    fn a_worker_limit_that_is_not_a_number_fails() {
        FixtureReports::workers_under(Some(OsString::from("many")));
    }
}
