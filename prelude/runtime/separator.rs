pub trait Separator: Sized {
    type Parts<'a>: Iterator<Item = &'a str>;

    fn split<'a>(self, text: &'a str) -> Self::Parts<'a>;
}

impl<'s> Separator for &'s str {
    type Parts<'a> = std::str::Split<'a, &'s str>;

    fn split<'a>(self, text: &'a str) -> Self::Parts<'a> {
        text.split(self)
    }
}

impl<'s> Separator for &'s String {
    type Parts<'a> = std::str::Split<'a, &'s String>;

    fn split<'a>(self, text: &'a str) -> Self::Parts<'a> {
        text.split(self)
    }
}

impl Separator for char {
    type Parts<'a> = std::str::Split<'a, char>;

    fn split<'a>(self, text: &'a str) -> Self::Parts<'a> {
        text.split(self)
    }
}

impl<F: FnMut(char) -> bool> Separator for F {
    type Parts<'a> = std::str::Split<'a, F>;

    fn split<'a>(self, text: &'a str) -> Self::Parts<'a> {
        text.split(self)
    }
}
