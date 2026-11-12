use crate::shape::Shape;

#[derive(Clone, Copy, Debug)]
pub struct ValueKind {
    names: &'static [&'static str],
    printed_as: Option<Shape>,
}

impl ValueKind {
    pub const NUMBER: Self = Self::printed_as(&["n", "k"], "{source}");
    pub const NUMBERS: Self = Self::printed_as(&["v", "w", "u"], "(len {source})");
    pub const CAR: Self = Self::printed_as(&["c", "d"], "(:year {source})");
    pub const ELEMENT_BORROW: Self = Self::printed_as(&["r", "s"], "{source}");
    pub const MAKE_BORROW: Self = Self::printed_as(&["m", "label"], "{source}");
    pub const TAG: Self = Self::printed_as(&["t"], "(:label {source})");
    pub const HELD_BORROWS: Self = Self::printed_as(&["held"], "(at {source} 0)");
    pub const READER: Self = Self::printed_as(&["size-of"], "({source})");
    pub const CHANGER: Self = Self::unprinted(&["grow-it"]);
    pub const FIGURE: Self = Self::printed_as(&["fig", "form"], "(size {source})");
    pub const MAP: Self = Self::printed_as(&["table", "index"], "(len {source})");
    pub const MAP_ENTRY: Self = Self::printed_as(&["entry"], "(unwrap-or {source} (ref 0))");
    pub const BORROWING_CLOSURE: Self = Self::unprinted(&["first-in"]);
    pub const ALL: [Self; 13] = [
        Self::NUMBER,
        Self::NUMBERS,
        Self::CAR,
        Self::ELEMENT_BORROW,
        Self::MAKE_BORROW,
        Self::TAG,
        Self::HELD_BORROWS,
        Self::READER,
        Self::CHANGER,
        Self::FIGURE,
        Self::MAP,
        Self::MAP_ENTRY,
        Self::BORROWING_CLOSURE,
    ];

    const fn printed_as(names: &'static [&'static str], shape: &'static str) -> Self {
        Self {
            names,
            printed_as: Some(Shape::of(shape)),
        }
    }

    const fn unprinted(names: &'static [&'static str]) -> Self {
        Self {
            names,
            printed_as: None,
        }
    }

    pub fn names(self) -> &'static [&'static str] {
        self.names
    }

    pub fn printed(self, name: &str) -> Option<String> {
        self.printed_as.map(|shape| shape.filled(name))
    }
}

impl PartialEq for ValueKind {
    fn eq(&self, other: &Self) -> bool {
        self.names == other.names
    }
}

impl Eq for ValueKind {}

#[cfg(test)]
mod tests {
    use super::ValueKind;

    #[test]
    fn prints_a_local_by_its_kind_and_never_calls_a_changer() {
        assert_eq!(ValueKind::TAG.printed("t"), Some("(:label t)".to_string()));
        assert_eq!(ValueKind::CHANGER.printed("grow-it"), None);
    }

    #[test]
    fn each_kind_is_told_apart_by_its_names() {
        let mut names: Vec<&str> = ValueKind::ALL
            .iter()
            .flat_map(|kind| kind.names())
            .copied()
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
        assert_ne!(ValueKind::NUMBER, ValueKind::ELEMENT_BORROW);
    }
}
