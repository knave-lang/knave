use super::piece::Piece;
use super::piece_literals::PieceLiterals;

pub struct BinderRenames<'r> {
    renames: &'r [(String, String)],
}

impl<'r> BinderRenames<'r> {
    pub fn new(renames: &'r [(String, String)]) -> Self {
        Self { renames }
    }
}

impl PieceLiterals for BinderRenames<'_> {
    fn literal(&self, piece: &Piece) -> Option<String> {
        let Piece::BinderPlaceholder { name } = piece else {
            return None;
        };
        let fresh = self
            .renames
            .iter()
            .find(|(binder, _)| binder == name)
            .map_or(name.as_str(), |(_, fresh)| fresh.as_str());
        Some(fresh.to_string())
    }
}
