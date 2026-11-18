#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lexeme<'t> {
    Argument(usize),
    Number(&'t str),
    Word(&'t str),
    Text,
    Symbol(char),
}
