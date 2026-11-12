#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Crate,
    Public,
}

impl Visibility {
    pub fn keyword(self) -> &'static str {
        match self {
            Visibility::Private => "",
            Visibility::Crate => "pub(crate) ",
            Visibility::Public => "pub ",
        }
    }
}
