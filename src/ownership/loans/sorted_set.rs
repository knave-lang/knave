#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortedSet<T> {
    items: Vec<T>,
}

impl<T> Default for SortedSet<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T: Ord + Copy> SortedSet<T> {
    pub fn insert(&mut self, item: T) {
        if let Err(position) = self.items.binary_search(&item) {
            self.items.insert(position, item);
        }
    }

    pub fn contains(&self, item: &T) -> bool {
        self.items.binary_search(item).is_ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.items.iter().copied()
    }
}

impl<T: Ord + Copy> Extend<T> for SortedSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        for item in items {
            self.insert(item);
        }
    }
}

impl<T: Ord + Copy> FromIterator<T> for SortedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut set = Self::default();
        set.extend(items);
        set
    }
}

#[cfg(test)]
mod tests {
    use super::SortedSet;

    #[test]
    fn keeps_each_item_once_in_order() {
        let set: SortedSet<u8> = [3, 1, 3, 2, 1].into_iter().collect();
        assert_eq!(set.iter().collect::<Vec<_>>(), [1, 2, 3]);
        assert!(set.contains(&2));
        assert!(!set.contains(&4));
    }
}
