use super::assigned_place::AssignedPlace;

pub enum FieldReadUse<'n> {
    Call(&'n str),
    Assignment(AssignedPlace<'n>),
    UnwrittenAssignment,
    Unnamed,
}
