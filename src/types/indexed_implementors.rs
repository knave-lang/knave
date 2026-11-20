use rustc_hash::FxHashMap;

use super::trait_rows_reach::Implementor;
use super::ty::{Scheme, Type};
use crate::core::{BindingId, PrimitiveType};

#[derive(Default)]
pub struct IndexedImplementors {
    implementors: Vec<Implementor>,
    every: Vec<usize>,
    open: Vec<usize>,
    by_head: FxHashMap<TypeHead, Vec<usize>>,
}

impl IndexedImplementors {
    pub fn new(implementors: Vec<Implementor>) -> Self {
        let mut indexed = Self {
            implementors: Vec::with_capacity(implementors.len()),
            every: Vec::with_capacity(implementors.len()),
            open: Vec::new(),
            by_head: FxHashMap::default(),
        };
        for implementor in implementors {
            indexed.push(implementor);
        }
        indexed
    }

    pub fn push(&mut self, implementor: Implementor) {
        let position = self.implementors.len();
        match TypeHead::of(&implementor.scheme.body) {
            Some(head) => self.push_headed(head, position),
            None => self.push_open(position),
        }
        self.every.push(position);
        self.implementors.push(implementor);
    }

    fn push_headed(&mut self, head: TypeHead, position: usize) {
        let open = &self.open;
        self.by_head
            .entry(head)
            .or_insert_with(|| open.clone())
            .push(position);
    }

    fn push_open(&mut self, position: usize) {
        self.open.push(position);
        for positions in self.by_head.values_mut() {
            positions.push(position);
        }
    }

    pub fn as_slice(&self) -> &[Implementor] {
        &self.implementors
    }

    pub fn candidates(&self, target: &Type) -> impl Iterator<Item = &Implementor> {
        let positions = match TypeHead::of(target) {
            Some(head) => self.by_head.get(&head).unwrap_or(&self.open),
            None => &self.every,
        };
        positions
            .iter()
            .map(|position| &self.implementors[*position])
    }

    pub fn retained(self, keep: impl FnMut(&Implementor) -> bool) -> Self {
        Self::new(self.implementors.into_iter().filter(keep).collect())
    }

    pub fn set_scheme(&mut self, body: &Type, scheme: Scheme) {
        let Some(implementor) = self
            .implementors
            .iter_mut()
            .find(|implementor| implementor.scheme.body == *body)
        else {
            return;
        };
        let moves_head = TypeHead::of(&scheme.body) != TypeHead::of(body);
        implementor.scheme = scheme;
        if moves_head {
            *self = Self::new(std::mem::take(&mut self.implementors));
        }
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
    use crate::core::PrimitiveType;
    use crate::types::{GenericId, Implementor, Scheme, Type, TypeVar};

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
        let indexed = IndexFixture::indexed(&[IndexFixture::integer(), IndexFixture::text()])
            .retained(|row| row.scheme.body != IndexFixture::integer());
        assert!(IndexFixture::candidates(&indexed, &IndexFixture::integer()).is_empty());
    }
}
