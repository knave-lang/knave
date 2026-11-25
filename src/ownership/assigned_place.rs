pub struct AssignedPlace<'n> {
    pub written: String,
    pub locals: Vec<&'n str>,
}
