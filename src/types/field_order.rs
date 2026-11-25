use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum FieldOrder {
    Declared,
    Written(Rc<[usize]>),
}

impl FieldOrder {
    pub fn of(fields: Vec<usize>) -> Self {
        if fields.is_sorted() {
            Self::Declared
        } else {
            Self::Written(fields.into())
        }
    }

    pub fn field_of(&self, written: usize) -> usize {
        match self {
            Self::Declared => written,
            Self::Written(fields) => fields[written],
        }
    }

    pub fn in_field_order<'v, T>(&self, written: &'v [T]) -> Vec<&'v T> {
        let mut values: Vec<(usize, &T)> = written
            .iter()
            .enumerate()
            .map(|(position, value)| (self.field_of(position), value))
            .collect();
        values.sort_by_key(|(field, _)| *field);
        values.into_iter().map(|(_, value)| value).collect()
    }
}
