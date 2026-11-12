#[derive(Clone, Copy, Debug)]
pub struct Shape(&'static str);

impl Shape {
    const HOLE: &'static str = "{source}";

    pub const PRINTED: Self = Self("(println {source})");
    pub const BORROWED: Self = Self("(ref {source})");
    pub const SIZE_BY_VARIANT: Self =
        Self("(match {source} ((figure/circle r) r) ((figure/square w) w) (figure/point 0))");

    pub const fn of(text: &'static str) -> Self {
        Self(text)
    }

    pub fn filled(&self, source: &str) -> String {
        self.0.replace(Self::HOLE, source)
    }
}

#[cfg(test)]
mod tests {
    use super::Shape;

    #[test]
    fn fills_its_hole_with_a_source() {
        assert_eq!(Shape::of("(len {source})").filled("v"), "(len v)");
        assert_eq!(Shape::PRINTED.filled("(len v)"), "(println (len v))");
    }
}
