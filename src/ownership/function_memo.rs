use std::cell::RefCell;

use rustc_hash::FxHashMap;

use crate::core::BindingId;

pub struct FunctionMemo<T> {
    known: RefCell<FxHashMap<BindingId, T>>,
}

impl<T> Default for FunctionMemo<T> {
    fn default() -> Self {
        Self {
            known: RefCell::default(),
        }
    }
}

impl<T: Clone> FunctionMemo<T> {
    pub fn remembered(&self, function: BindingId, compute: impl FnOnce() -> T) -> T {
        if let Some(known) = self.known.borrow().get(&function) {
            return known.clone();
        }
        let computed = compute();
        self.known.borrow_mut().insert(function, computed.clone());
        computed
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::FunctionMemo;
    use crate::core::BindingId;

    #[test]
    fn a_function_is_computed_once_and_then_remembered() {
        let memo = FunctionMemo::default();
        let computed = Cell::new(0);
        let compute = || {
            computed.set(computed.get() + 1);
            computed.get()
        };
        let function = BindingId::from_index(1);
        assert_eq!(memo.remembered(function, compute), 1);
        assert_eq!(memo.remembered(function, compute), 1);
        assert_eq!(computed.get(), 1);
    }
}
