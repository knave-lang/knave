use std::cell::Cell;

#[derive(Default)]
pub(super) struct UseMark {
    used: Cell<bool>,
}

impl UseMark {
    pub(super) fn mark(&self) {
        self.used.set(true);
    }

    pub(super) fn is_marked(&self) -> bool {
        self.used.get()
    }
}
