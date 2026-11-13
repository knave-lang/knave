#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct SectionNames {
    pub(super) libraries: String,
    pub(super) dependencies: String,
    pub(super) dev_dependencies: String,
}
