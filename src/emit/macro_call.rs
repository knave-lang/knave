use super::identifier::Identifier;

pub struct MacroCall<'a> {
    name: &'a str,
}

impl<'a> MacroCall<'a> {
    const BANG: char = '!';

    pub fn ending(written_before_parenthesis: &'a str) -> Option<Self> {
        let path = written_before_parenthesis
            .trim_end()
            .strip_suffix(Self::BANG)?;
        Some(Self {
            name: Identifier::ending(path).text(),
        })
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}
