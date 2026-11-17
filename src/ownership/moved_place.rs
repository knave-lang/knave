use std::rc::Rc;

use crate::core::BindingArena;
use crate::types::FieldPlace;

#[derive(Clone)]
pub struct MovedPlace {
    written: Rc<str>,
    borrowed: Option<Rc<str>>,
}

impl MovedPlace {
    pub fn of(place: &FieldPlace, bindings: &BindingArena) -> Self {
        let written = place.written(bindings).into();
        let borrowed = Some(place.written_borrowed(bindings).into());
        Self { written, borrowed }
    }

    pub fn whole(name: &str) -> Self {
        Self {
            written: name.into(),
            borrowed: Some(FieldPlace::borrowed_root(name).into()),
        }
    }

    pub fn owned(self) -> Self {
        Self {
            borrowed: None,
            ..self
        }
    }

    pub fn written(&self) -> &str {
        &self.written
    }

    pub fn borrowed(&self) -> Option<&str> {
        self.borrowed.as_deref()
    }
}
