pub(super) trait SavedKey<'a>: Copy {
    fn text(self) -> &'a str;

    fn first_slot(self, mask: usize) -> usize;
}
