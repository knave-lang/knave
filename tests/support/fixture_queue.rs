use std::any::Any;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use rustc_hash::FxHashMap;

type PanicPayload = Box<dyn Any + Send>;

pub struct FixtureQueue {
    state: Mutex<QueueState>,
    progress: Condvar,
}

impl FixtureQueue {
    const WATCH_INTERVAL: Duration = Duration::from_millis(100);

    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            state: Mutex::new(QueueState::of(paths)),
            progress: Condvar::new(),
        }
    }

    pub fn drain(&self, report: &impl Fn(&Path) -> String) {
        while let Some(path) = self.start_next() {
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| report(&path)));
            self.finish(path, outcome);
        }
    }

    pub fn finished_reports(&self, deadline: Duration) -> FxHashMap<PathBuf, String> {
        match self.awaited_outcome(deadline) {
            Outcome::Reports(reports) => reports,
            Outcome::Panicked(payload) => panic::resume_unwind(payload),
            Outcome::Overdue(path) => panic!(
                "the fixture {} ran longer than {deadline:?}",
                path.display()
            ),
        }
    }

    fn awaited_outcome(&self, deadline: Duration) -> Outcome {
        let mut state = self.lock();
        loop {
            if let Some(outcome) = state.outcome(deadline) {
                return outcome;
            }
            state = self
                .progress
                .wait_timeout(state, Self::WATCH_INTERVAL)
                .expect("no thread panics while holding the queue")
                .0;
        }
    }

    fn start_next(&self) -> Option<PathBuf> {
        self.lock().start_next()
    }

    fn finish(&self, path: PathBuf, outcome: Result<String, PanicPayload>) {
        self.lock().record(path, outcome);
        self.progress.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, QueueState> {
        self.state
            .lock()
            .expect("no thread panics while holding the queue")
    }
}

enum Outcome {
    Reports(FxHashMap<PathBuf, String>),
    Panicked(PanicPayload),
    Overdue(PathBuf),
}

struct QueueState {
    pending: Vec<PathBuf>,
    running: FxHashMap<PathBuf, Instant>,
    finished: FxHashMap<PathBuf, String>,
    panicked: Option<PanicPayload>,
}

impl QueueState {
    fn of(paths: Vec<PathBuf>) -> Self {
        Self {
            pending: paths,
            running: FxHashMap::default(),
            finished: FxHashMap::default(),
            panicked: None,
        }
    }

    fn start_next(&mut self) -> Option<PathBuf> {
        let path = self.pending.pop()?;
        self.running.insert(path.clone(), Instant::now());
        Some(path)
    }

    fn record(&mut self, path: PathBuf, outcome: Result<String, PanicPayload>) {
        self.running.remove(&path);
        match outcome {
            Ok(report) => {
                self.finished.insert(path, report);
            }
            Err(payload) => {
                self.panicked.get_or_insert(payload);
            }
        }
    }

    fn outcome(&mut self, deadline: Duration) -> Option<Outcome> {
        if let Some(payload) = self.panicked.take() {
            return Some(Outcome::Panicked(payload));
        }
        if let Some(path) = self.overdue(deadline) {
            self.pending.clear();
            return Some(Outcome::Overdue(path));
        }
        self.is_done()
            .then(|| Outcome::Reports(mem::take(&mut self.finished)))
    }

    fn overdue(&self, deadline: Duration) -> Option<PathBuf> {
        self.running
            .iter()
            .find(|(_, started)| started.elapsed() > deadline)
            .map(|(path, _)| path.clone())
    }

    fn is_done(&self) -> bool {
        self.pending.is_empty() && self.running.is_empty()
    }
}
