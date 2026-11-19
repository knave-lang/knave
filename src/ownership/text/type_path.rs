use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypePath(Vec<usize>);

impl TypePath {
    pub fn is_top(&self) -> bool {
        self.0.is_empty()
    }

    pub fn child(&self, part: usize) -> Self {
        let mut parts = self.0.clone();
        parts.push(part);
        Self(parts)
    }

    pub fn below(&self, part: usize) -> Option<Self> {
        match self.0.split_first() {
            Some((first, rest)) if *first == part => Some(Self(rest.to_vec())),
            _ => None,
        }
    }

    pub fn joined(&self, inner: &Self) -> Self {
        Self(self.0.iter().chain(&inner.0).copied().collect())
    }
}

impl fmt::Display for TypePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|part| write!(formatter, "[{part}]"))
    }
}

#[cfg(test)]
mod tests {
    use super::TypePath;

    #[test]
    fn a_joined_path_runs_outer_parts_first() {
        let outer = TypePath::default().child(1);
        let inner = TypePath::default().child(0);
        assert_eq!(outer.joined(&inner).to_string(), "[1][0]");
    }

    #[test]
    fn a_path_below_its_first_part_drops_that_part() {
        let path = TypePath::default().child(1).child(0);
        assert_eq!(path.below(1), Some(TypePath::default().child(0)));
        assert_eq!(path.below(0), None);
        assert_eq!(TypePath::default().below(0), None);
    }

    #[test]
    fn the_top_path_prints_nothing() {
        assert!(TypePath::default().is_top());
        assert_eq!(TypePath::default().to_string(), "");
    }
}
