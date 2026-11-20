#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Literal {
    Outside,
    String,
    Character,
}

impl Literal {
    pub fn closes(self, character: char) -> bool {
        match self {
            Literal::Outside => false,
            Literal::String => character == '"',
            Literal::Character => character == '\'',
        }
    }
}
