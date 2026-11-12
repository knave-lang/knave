pub trait Contains<Part: ?Sized> {
    fn contains(&self, part: &Part) -> bool;
}

impl Contains<&str> for str {
    fn contains(&self, part: &&str) -> bool {
        str::contains(self, *part)
    }
}

impl Contains<&str> for String {
    fn contains(&self, part: &&str) -> bool {
        self.as_str().contains(*part)
    }
}

impl<T: PartialEq> Contains<T> for [T] {
    fn contains(&self, part: &T) -> bool {
        <[T]>::contains(self, part)
    }
}

impl<T: PartialEq> Contains<T> for Vec<T> {
    fn contains(&self, part: &T) -> bool {
        self.as_slice().contains(part)
    }
}

impl<Part: ?Sized, T: Contains<Part> + ?Sized> Contains<Part> for &T {
    fn contains(&self, part: &Part) -> bool {
        T::contains(self, part)
    }
}
