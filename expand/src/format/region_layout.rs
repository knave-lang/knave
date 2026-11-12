#[derive(Debug, PartialEq, Eq)]
pub enum RegionLayout {
    LaidOut(Vec<String>),
    Unreadable(usize),
    Prose,
}
