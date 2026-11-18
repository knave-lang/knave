#[derive(Clone, Copy)]
pub enum PassingRelaxation {
    Copied,
    SharedReference,
    Exact,
}
