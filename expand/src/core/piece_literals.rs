use super::piece::Piece;

pub trait PieceLiterals {
    fn literal(&self, piece: &Piece) -> Option<String>;
}
