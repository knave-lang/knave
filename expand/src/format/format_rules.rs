use super::docstring_syntax::DocstringSyntax;
use super::shapes::ShapeCatalog;

#[derive(Clone, Copy)]
pub struct FormatRules<'s> {
    pub shapes: &'s ShapeCatalog,
    pub syntax: &'s DocstringSyntax,
}
