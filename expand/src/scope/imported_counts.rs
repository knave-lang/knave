#[derive(Debug, PartialEq)]
pub(crate) struct ImportedCounts {
    pub(crate) tables: usize,
    pub(crate) modules: usize,
    pub(crate) decoded_names: usize,
}
