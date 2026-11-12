use super::shapes::{ShapeCatalog, ShapeStatement};
use crate::reader::Reader;
use crate::source::SourceMap;

pub struct ShapeCatalogFixture;

impl ShapeCatalogFixture {
    pub fn stated(text: &str) -> ShapeCatalog {
        let mut sources = SourceMap::default();
        let source = sources.add("shapes", text);
        let forms = Reader::new(sources.file(source))
            .read()
            .unwrap_or_else(|_| panic!("the shape statements read"));
        let mut shapes = ShapeCatalog::default();
        forms
            .iter()
            .filter_map(ShapeStatement::of)
            .for_each(|statement| shapes.add(&statement));
        shapes
    }
}
