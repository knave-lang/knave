pub struct TemplateBraces;

impl TemplateBraces {
    pub const OPEN: char = '{';
    pub const CLOSE: char = '}';

    pub fn appear_in(text: &str) -> bool {
        text.contains([Self::OPEN, Self::CLOSE])
    }
}
