use super::inline_templates::InlineTemplates;
use super::templates::TemplateBorrows;

pub trait InlineSchemes {
    fn template_borrows() -> TemplateBorrows;
}

pub struct DroppedInlineSchemes;

pub struct KeptInlineSchemes;

impl InlineSchemes for DroppedInlineSchemes {
    fn template_borrows() -> TemplateBorrows {
        TemplateBorrows::default()
    }
}

impl InlineSchemes for KeptInlineSchemes {
    fn template_borrows() -> TemplateBorrows {
        TemplateBorrows::from(InlineTemplates::keeping_schemes())
    }
}
