#[derive(Clone, Copy)]
pub struct Call<'text> {
    pub caller: &'text str,
    pub callee: &'text str,
    pub cost: u64,
}
