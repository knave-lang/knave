#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueKind {
    names: &'static [&'static str],
    printed_as: Option<&'static str>,
}

impl ValueKind {
    const NAME: &'static str = "{name}";

    pub const NUMBER: Self = Self::printed(&["n", "k"], "{name}");
    pub const NUMBERS: Self = Self::printed(&["v", "w", "u"], "(len {name})");
    pub const CAR: Self = Self::printed(&["c", "d"], "(:year {name})");
    pub const ELEMENT_BORROW: Self = Self::printed(&["r", "s"], "{name}");
    pub const MAKE_BORROW: Self = Self::printed(&["m", "label"], "{name}");
    pub const TAG: Self = Self::printed(&["t"], "(:label {name})");
    pub const HELD_BORROWS: Self = Self::printed(&["held"], "(at {name} 0)");
    pub const READER: Self = Self::printed(&["size-of"], "({name})");
    pub const CHANGER: Self = Self {
        names: &["grow-it"],
        printed_as: None,
    };
    pub const ALL: [Self; 9] = [
        Self::NUMBER,
        Self::NUMBERS,
        Self::CAR,
        Self::ELEMENT_BORROW,
        Self::MAKE_BORROW,
        Self::TAG,
        Self::HELD_BORROWS,
        Self::READER,
        Self::CHANGER,
    ];

    const fn printed(names: &'static [&'static str], shape: &'static str) -> Self {
        Self {
            names,
            printed_as: Some(shape),
        }
    }

    pub fn names(self) -> &'static [&'static str] {
        self.names
    }

    pub fn print_of(self, name: &str) -> Option<String> {
        self.printed_as
            .map(|shape| format!("(println {})", shape.replace(Self::NAME, name)))
    }
}

#[cfg(test)]
mod tests {
    use super::ValueKind;

    #[test]
    fn prints_a_local_by_its_kind_and_never_calls_a_changer() {
        assert_eq!(
            ValueKind::TAG.print_of("t"),
            Some("(println (:label t))".to_string())
        );
        assert_eq!(ValueKind::CHANGER.print_of("grow-it"), None);
    }

    #[test]
    fn no_two_kinds_share_a_name() {
        let mut names: Vec<&str> = ValueKind::ALL
            .iter()
            .flat_map(|kind| kind.names())
            .copied()
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
    }
}
