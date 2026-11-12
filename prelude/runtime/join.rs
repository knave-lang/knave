pub trait Join {
    fn join(&self, separator: &str) -> String;
}

impl<S: std::borrow::Borrow<str>> Join for [S] {
    fn join(&self, separator: &str) -> String {
        <[S]>::join(self, separator)
    }
}

impl<S: std::borrow::Borrow<str>> Join for Vec<S> {
    fn join(&self, separator: &str) -> String {
        self.as_slice().join(separator)
    }
}
