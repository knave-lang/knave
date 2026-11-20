use std::cell::OnceCell;

use rustc_hash::FxHashMap;

use super::implementor::Implementor;
use super::ty::{Scheme, Type};
use crate::core::{BindingId, PrimitiveType};

pub struct IndexedImplementors {
    implementors: Vec<Implementor>,
    index: OnceCell<HeadIndex>,
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

    pub fn candidates(&self, target: &Type) -> Candidates<'_> {
        let Some(head) = TypeHead::of(target) else {
            return Candidates::Every(self.implementors.iter());
        };
        let index = self.index.get_or_init(|| HeadIndex::of(&self.implementors));
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
    open: Vec<usize>,
    by_head: FxHashMap<TypeHead, Vec<usize>>,
}

impl HeadIndex {
    fn of(implementors: &[Implementor]) -> Self {
        let mut index = Self {
            open: Vec::new(),
            by_head: FxHashMap::default(),
        };
        for (position, implementor) in implementors.iter().enumerate() {
            match TypeHead::of(&implementor.scheme.body) {
                Some(head) => index.by_head.entry(head).or_default().push(position),
                None => index.open.push(position),
            }
        }
        index
    }

    fn positions(&self, head: TypeHead) -> MergedPositions<'_> {
        MergedPositions {
            headed: self.by_head.get(&head).map_or(&[], Vec::as_slice),
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
    headed: &'i [usize],
    open: &'i [usize],
}

impl Iterator for MergedPositions<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let takes_headed = match (self.headed.first(), self.open.first()) {
            (Some(headed), Some(open)) => headed < open,
            (headed, _) => headed.is_some(),
        };
        let side = if takes_headed {
            &mut self.headed
        } else {
            &mut self.open
        };
        let (position, rest) = side.split_first()?;
        *side = rest;
        Some(*position)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
            IndexFixture::integer(),
            Type::Named(named, Vec::new()),
            Type::TraitObject(named),
            Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit())),
            Type::shared(IndexFixture::integer()),
            Type::mutable(IndexFixture::integer()),
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
    fn a_dropped_row_leaves_the_index() {
        let mut indexed = IndexFixture::indexed(&[IndexFixture::integer(), IndexFixture::text()]);
        indexed.retain(|row| row.scheme.body != IndexFixture::integer());
        assert!(IndexFixture::candidates(&indexed, &IndexFixture::integer()).is_empty());
    }
}
