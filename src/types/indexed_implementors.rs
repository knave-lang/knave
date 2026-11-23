use std::cell::OnceCell;

use super::implementor::Implementor;
use super::ty::{Scheme, Type};
use crate::core::{BindingId, PrimitiveType};

pub struct IndexedImplementors {
    implementors: Vec<Implementor>,
    index: OnceCell<Box<HeadIndex>>,
}

impl IndexedImplementors {
    pub fn new(implementors: Vec<Implementor>) -> Self {
        Self {
            implementors,
            index: OnceCell::new(),
        }
    }

    pub fn push(&mut self, implementor: Implementor) {
        self.implementors.push(implementor);
        self.index.take();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Implementor> {
        self.implementors.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.implementors.is_empty()
    }

    pub fn candidates(&self, target: &Type) -> Candidates<'_> {
        let Some(head) = TypeHead::of(target) else {
            return Candidates::Every(self.implementors.iter());
        };
        let index = self
            .index
            .get_or_init(|| Box::new(HeadIndex::of(&self.implementors)));
        Candidates::Headed {
            implementors: &self.implementors,
            positions: index.positions(head),
        }
    }

    pub fn retain(&mut self, keep: impl FnMut(&Implementor) -> bool) {
        self.implementors.retain(keep);
        self.index.take();
    }

    pub fn set_scheme(&mut self, body: &Type, scheme: Scheme) {
        if let Some(implementor) = self
            .implementors
            .iter_mut()
            .find(|implementor| implementor.scheme.body == *body)
        {
            implementor.scheme = scheme;
            self.index.take();
        }
    }
}

struct HeadIndex {
    headed: Vec<(TypeHead, usize)>,
    open: Vec<usize>,
}

impl HeadIndex {
    fn of(implementors: &[Implementor]) -> Self {
        let mut index = Self {
            headed: Vec::with_capacity(implementors.len()),
            open: Vec::new(),
        };
        for (position, implementor) in implementors.iter().enumerate() {
            match TypeHead::of(&implementor.scheme.body) {
                Some(head) => index.headed.push((head, position)),
                None => index.open.push(position),
            }
        }
        index.headed.sort_unstable();
        index
    }

    fn positions(&self, head: TypeHead) -> MergedPositions<'_> {
        let start = self.headed.partition_point(|(entry, _)| *entry < head);
        let end = self.headed.partition_point(|(entry, _)| *entry <= head);
        MergedPositions {
            headed: &self.headed[start..end],
            open: &self.open,
        }
    }
}

pub enum Candidates<'i> {
    Every(std::slice::Iter<'i, Implementor>),
    Headed {
        implementors: &'i [Implementor],
        positions: MergedPositions<'i>,
    },
}

impl<'i> Iterator for Candidates<'i> {
    type Item = &'i Implementor;

    fn next(&mut self) -> Option<&'i Implementor> {
        match self {
            Self::Every(rows) => rows.next(),
            Self::Headed {
                implementors,
                positions,
            } => positions.next().map(|position| &implementors[position]),
        }
    }
}

pub struct MergedPositions<'i> {
    headed: &'i [(TypeHead, usize)],
    open: &'i [usize],
}

impl Iterator for MergedPositions<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        match (self.headed.split_first(), self.open.split_first()) {
            (Some((&(_, headed), rest)), Some((&open, _))) if headed < open => {
                self.headed = rest;
                Some(headed)
            }
            (_, Some((&open, rest))) => {
                self.open = rest;
                Some(open)
            }
            (Some((&(_, headed), rest)), None) => {
                self.headed = rest;
                Some(headed)
            }
            (None, None) => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TypeHead {
    Primitive(PrimitiveType),
    Named(BindingId),
    TraitObject(BindingId),
    Function,
    Ref,
    Mut,
}

impl TypeHead {
    fn of(ty: &Type) -> Option<Self> {
        match ty {
            Type::Var(_) | Type::Generic(_) => None,
            Type::Primitive(primitive) => Some(Self::Primitive(*primitive)),
            Type::Named(binding, _) => Some(Self::Named(*binding)),
            Type::TraitObject(binding) => Some(Self::TraitObject(*binding)),
            Type::Function(..) => Some(Self::Function),
            Type::Ref(..) => Some(Self::Ref),
            Type::Mut(..) => Some(Self::Mut),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IndexedImplementors;
    use crate::core::{BindingId, PrimitiveType};
    use crate::types::{ClosureKind, GenericId, Implementor, Scheme, Type, TypeVar};

    struct IndexFixture;

    impl IndexFixture {
        fn indexed(bodies: &[Type]) -> IndexedImplementors {
            IndexedImplementors::new(bodies.iter().cloned().map(Self::row).collect())
        }

        fn row(body: Type) -> Implementor {
            Implementor {
                scheme: Scheme::monomorphic(body),
                argument: None,
            }
        }

        fn candidates(indexed: &IndexedImplementors, target: &Type) -> Vec<Type> {
            indexed
                .candidates(target)
                .map(|row| row.scheme.body.clone())
                .collect()
        }

        fn integer() -> Type {
            Type::Primitive(PrimitiveType::I64)
        }

        fn text() -> Type {
            Type::Primitive(PrimitiveType::Str)
        }

        fn blanket() -> Type {
            Type::Generic(GenericId::new(0))
        }
    }

    #[test]
    fn a_closed_target_meets_its_own_head_and_the_open_rows_in_written_order() {
        let indexed = IndexFixture::indexed(&[
            IndexFixture::blanket(),
            IndexFixture::text(),
            IndexFixture::integer(),
        ]);
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::integer()),
            [IndexFixture::blanket(), IndexFixture::integer()]
        );
    }

    #[test]
    fn each_kind_of_head_meets_only_its_own_rows() {
        let named = BindingId::from_index(0);
        let bodies = [
            Type::mutable(IndexFixture::integer()),
            Type::shared(IndexFixture::integer()),
            Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit())),
            Type::TraitObject(named),
            Type::Named(named, Vec::new()),
            IndexFixture::integer(),
        ];
        let indexed = IndexFixture::indexed(&bodies);
        for body in &bodies {
            assert_eq!(
                IndexFixture::candidates(&indexed, body),
                std::slice::from_ref(body)
            );
        }
    }

    #[test]
    fn a_head_with_no_row_meets_only_the_open_rows() {
        let indexed = IndexFixture::indexed(&[IndexFixture::text(), IndexFixture::blanket()]);
        let unit = Type::unit();
        assert_eq!(
            IndexFixture::candidates(&indexed, &unit),
            [IndexFixture::blanket()]
        );
    }

    #[test]
    fn an_open_target_meets_every_row() {
        let bodies = [IndexFixture::text(), IndexFixture::integer()];
        let indexed = IndexFixture::indexed(&bodies);
        let open = Type::Var(TypeVar::new(0));
        assert_eq!(IndexFixture::candidates(&indexed, &open), bodies);
    }

    #[test]
    fn an_open_row_added_later_joins_each_head() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer()]);
        indexed.push(IndexFixture::row(IndexFixture::blanket()));
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::integer()),
            [IndexFixture::integer(), IndexFixture::blanket()]
        );
    }

    #[test]
    fn a_scheme_that_moves_a_row_to_another_head_moves_its_index() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer()]);
        indexed.set_scheme(
            &IndexFixture::integer(),
            Scheme::monomorphic(IndexFixture::text()),
        );
        assert!(IndexFixture::candidates(&indexed, &IndexFixture::integer()).is_empty());
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::text()),
            [IndexFixture::text()]
        );
    }

    #[test]
    fn a_scheme_for_a_type_with_no_row_changes_nothing() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer()]);
        indexed.set_scheme(
            &IndexFixture::text(),
            Scheme::monomorphic(IndexFixture::text()),
        );
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::integer()),
            [IndexFixture::integer()]
        );
    }

    #[test]
    fn a_row_pushed_after_a_lookup_joins_the_next_lookup() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer()]);
        IndexFixture::candidates(&indexed, &IndexFixture::integer());
        indexed.push(IndexFixture::row(IndexFixture::blanket()));
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::integer()),
            [IndexFixture::integer(), IndexFixture::blanket()]
        );
    }

    #[test]
    fn a_scheme_set_after_a_lookup_moves_the_next_lookup() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer()]);
        IndexFixture::candidates(&indexed, &IndexFixture::integer());
        indexed.set_scheme(
            &IndexFixture::integer(),
            Scheme::monomorphic(IndexFixture::text()),
        );
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::text()),
            [IndexFixture::text()]
        );
    }

    #[test]
    fn a_row_dropped_after_a_lookup_leaves_the_next_lookup() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::text(), IndexFixture::integer()]);
        IndexFixture::candidates(&indexed, &IndexFixture::integer());
        indexed.retain(|row| row.scheme.body != IndexFixture::text());
        assert_eq!(
            IndexFixture::candidates(&indexed, &IndexFixture::integer()),
            [IndexFixture::integer()]
        );
    }

    #[test]
    fn a_dropped_row_leaves_the_index() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer(), IndexFixture::text()]);
        indexed.retain(|row| row.scheme.body != IndexFixture::integer());
        assert!(IndexFixture::candidates(&indexed, &IndexFixture::integer()).is_empty());
    }
}
