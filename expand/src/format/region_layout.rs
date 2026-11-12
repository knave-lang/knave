use super::unreadable_code::UnreadableCode;

#[derive(Debug, PartialEq, Eq)]
pub enum RegionLayout {
    LaidOut(Vec<String>),
    Unreadable(UnreadableCode),
    Prose,
}
