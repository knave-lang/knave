pub trait EmbeddedSources {
    fn count(&self) -> usize;

    fn name(&self, position: usize) -> String;

    fn text(&self, position: usize) -> &'static str;
}
