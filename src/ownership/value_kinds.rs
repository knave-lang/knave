use super::functions::{ParamMode, ParamSlot};
use crate::types::CopyRules;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Moving,
    SmallCopy,
    LargeCopy,
}

impl ValueKind {
    pub fn of(slot: &ParamSlot, copy: &CopyRules) -> Self {
        if copy.is_small_copy(&slot.ty) {
            ValueKind::SmallCopy
        } else if copy.is_copy(&slot.ty) {
            ValueKind::LargeCopy
        } else {
            ValueKind::Moving
        }
    }

    pub fn starting_mode(self, rebuilt: bool) -> ParamMode {
        match self {
            ValueKind::SmallCopy | ValueKind::LargeCopy => ParamMode::Copy,
            ValueKind::Moving if rebuilt => ParamMode::Own,
            ValueKind::Moving => ParamMode::Ref,
        }
    }

    pub fn need(self, context: ParamMode) -> ParamMode {
        match (self, context) {
            (ValueKind::SmallCopy, ParamMode::Mut) => ParamMode::Mut,
            (ValueKind::SmallCopy, _) => ParamMode::Copy,
            _ => context,
        }
    }

    pub fn stronger(self, current: ParamMode, needed: ParamMode) -> ParamMode {
        let changes_caller = current == ParamMode::Mut || needed == ParamMode::Mut;
        match self {
            ValueKind::SmallCopy | ValueKind::LargeCopy if changes_caller => ParamMode::Mut,
            _ => current.max(needed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValueKind;
    use crate::ownership::functions::ParamMode;

    #[test]
    fn a_change_outranks_a_read_of_a_copy_value() {
        assert_eq!(
            ValueKind::LargeCopy.stronger(ParamMode::Own, ParamMode::Mut),
            ParamMode::Mut
        );
    }

    #[test]
    fn a_move_outranks_a_change_of_a_moving_value() {
        assert_eq!(
            ValueKind::Moving.stronger(ParamMode::Mut, ParamMode::Own),
            ParamMode::Own
        );
    }

    #[test]
    fn a_small_copy_value_needs_no_borrow_to_be_read() {
        assert_eq!(ValueKind::SmallCopy.need(ParamMode::Ref), ParamMode::Copy);
    }

    #[test]
    fn a_large_copy_value_keeps_a_shared_borrow() {
        assert_eq!(ValueKind::LargeCopy.need(ParamMode::Ref), ParamMode::Ref);
    }
}
