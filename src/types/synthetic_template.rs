use super::template_ties::TemplateTies;

pub struct SyntheticTemplate {
    text: &'static str,
    lifetime: &'static [usize],
    held: &'static [usize],
}

impl SyntheticTemplate {
    pub const ITERATE: Self = Self::borrowing("{0}.iter()");
    pub const ITERATE_COPIED: Self = Self::borrowing("{0}.iter().copied()");
    pub const LOOK_UP: Self = Self::borrowing("{0}.get({1})");
    pub const KEYS: Self = Self::borrowing("{0}.keys()");
    pub const VALUES: Self = Self::borrowing("{0}.values()");
    pub const PUSH: Self = Self::untied("{0}.push({1})");
    pub const PUSH_TEXT: Self = Self::untied("{0}.push_str({1})");
    pub const MAP_UPDATE: Self = Self {
        text: "@map-update",
        lifetime: &[],
        held: &[0, 3],
    };

    pub const ALL: [&'static Self; 8] = [
        &Self::ITERATE,
        &Self::ITERATE_COPIED,
        &Self::LOOK_UP,
        &Self::KEYS,
        &Self::VALUES,
        &Self::PUSH,
        &Self::PUSH_TEXT,
        &Self::MAP_UPDATE,
    ];

    const fn borrowing(text: &'static str) -> Self {
        Self {
            text,
            lifetime: &[0],
            held: &[0],
        }
    }

    const fn untied(text: &'static str) -> Self {
        Self {
            text,
            lifetime: &[],
            held: &[],
        }
    }

    pub fn named(text: &str) -> Option<&'static Self> {
        Self::ALL.into_iter().find(|template| template.text == text)
    }

    pub fn text(&self) -> &'static str {
        self.text
    }

    pub fn ties(&self) -> TemplateTies {
        TemplateTies::new(self.lifetime, self.held)
    }
}
