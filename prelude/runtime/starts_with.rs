pub trait StartsWith {
    fn starts_with(&self, prefix: &str) -> bool;
}

impl StartsWith for str {
    fn starts_with(&self, prefix: &str) -> bool {
        str::starts_with(self, prefix)
    }
}

impl StartsWith for String {
    fn starts_with(&self, prefix: &str) -> bool {
        self.as_str().starts_with(prefix)
    }
}

impl<T: StartsWith + ?Sized> StartsWith for &T {
    fn starts_with(&self, prefix: &str) -> bool {
        T::starts_with(self, prefix)
    }
}
