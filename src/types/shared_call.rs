use super::callee_borrows::CalleeBorrows;
use super::shared_group::SharedGroup;

pub(super) struct SharedCall<'a> {
    pub(super) group: &'a SharedGroup,
    pub(super) callee: CalleeBorrows<'a>,
}
