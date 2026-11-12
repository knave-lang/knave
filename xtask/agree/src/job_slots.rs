use std::sync::{Condvar, Mutex, MutexGuard};

pub struct JobSlots {
    free: Mutex<usize>,
    released: Condvar,
}

pub struct JobSlot<'a> {
    slots: &'a JobSlots,
}

impl JobSlots {
    pub fn new(count: usize) -> Self {
        Self {
            free: Mutex::new(count.max(1)),
            released: Condvar::new(),
        }
    }

    pub fn take(&self) -> JobSlot<'_> {
        let mut free = self.lock();
        while *free == 0 {
            free = self
                .released
                .wait(free)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *free -= 1;
        JobSlot { slots: self }
    }

    fn give_back(&self) {
        *self.lock() += 1;
        self.released.notify_one();
    }

    fn lock(&self) -> MutexGuard<'_, usize> {
        self.free
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for JobSlot<'_> {
    fn drop(&mut self) {
        self.slots.give_back();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::JobSlots;

    struct SlotsFixture;

    impl SlotsFixture {
        const SLOTS: usize = 2;
        const THREADS: usize = 6;
        const HOLD: Duration = Duration::from_millis(20);
    }

    #[test]
    fn never_lets_more_jobs_run_than_its_slots() {
        let slots = JobSlots::new(SlotsFixture::SLOTS);
        let running = AtomicUsize::new(0);
        let most = AtomicUsize::new(0);
        thread::scope(|scope| {
            for _ in 0..SlotsFixture::THREADS {
                scope.spawn(|| {
                    let _slot = slots.take();
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    most.fetch_max(now, Ordering::SeqCst);
                    thread::sleep(SlotsFixture::HOLD);
                    running.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        assert_eq!(most.load(Ordering::SeqCst), SlotsFixture::SLOTS);
    }
}
