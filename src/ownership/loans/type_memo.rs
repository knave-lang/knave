use std::cell::RefCell;

use rustc_hash::FxHashMap;

use crate::types::Type;

#[derive(Default)]
pub struct TypeMemo {
    answers: RefCell<FxHashMap<Type, bool>>,
}

impl TypeMemo {
    pub fn answer(&self, ty: &Type, decide: impl FnOnce(&Type) -> bool) -> bool {
        if let Some(answer) = self.answers.borrow().get(ty) {
            return *answer;
        }
        let answer = decide(ty);
        self.answers.borrow_mut().insert(ty.clone(), answer);
        answer
    }
}
