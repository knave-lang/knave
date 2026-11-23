use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ty::{Bound, GenericId, Lifetime, Scheme, Type};

pub enum InlineScheme {
    One(Rc<Scheme>),
    Several,
}

impl InlineScheme {
    pub fn widened(self, other: &Scheme) -> Self {
        match self {
            Self::One(scheme) if SchemeShape::of(&scheme).matches(other) => Self::One(scheme),
            _ => Self::Several,
        }
    }

    pub fn single(&self) -> Option<&Scheme> {
        match self {
            Self::One(scheme) => Some(scheme),
            Self::Several => None,
        }
    }
}

struct SchemeShape<'s> {
    scheme: &'s Scheme,
    renaming: FxHashMap<GenericId, GenericId>,
}

impl<'s> SchemeShape<'s> {
    fn of(scheme: &'s Scheme) -> Self {
        Self {
            scheme,
            renaming: FxHashMap::default(),
        }
    }

    fn matches(mut self, other: &Scheme) -> bool {
        if self.scheme.params.len() != other.params.len() {
            return false;
        }
        self.renaming = self
            .scheme
            .params
            .iter()
            .zip(&other.params)
            .map(|(mine, theirs)| (mine.id, theirs.id))
            .collect();
        let bounds = self
            .scheme
            .params
            .iter()
            .zip(&other.params)
            .all(|(mine, theirs)| {
                mine.bounds.len() == theirs.bounds.len()
                    && mine
                        .bounds
                        .iter()
                        .zip(&theirs.bounds)
                        .all(|(mine, theirs)| self.same_bound(mine, theirs))
            });
        bounds && self.same_type(&self.scheme.body, &other.body)
    }

    fn same_bound(&self, mine: &Bound, theirs: &Bound) -> bool {
        mine.binding == theirs.binding
            && match (&mine.argument, &theirs.argument) {
                (Some(mine), Some(theirs)) => self.same_type(mine, theirs),
                (None, None) => true,
                _ => false,
            }
    }

    fn same_type(&self, mine: &Type, theirs: &Type) -> bool {
        match (mine, theirs) {
            (Type::Generic(mine), Type::Generic(theirs)) => self.renaming.get(mine) == Some(theirs),
            (Type::Named(mine, my_parts), Type::Named(theirs, their_parts)) => {
                mine == theirs && self.same_types(my_parts, their_parts)
            }
            (
                Type::Function(my_kind, my_parameters, my_result),
                Type::Function(their_kind, their_parameters, their_result),
            ) => {
                my_kind == their_kind
                    && self.same_types(my_parameters, their_parameters)
                    && self.same_type(my_result, their_result)
            }
            (Type::Ref(mine, my_referent), Type::Ref(theirs, their_referent))
            | (Type::Mut(mine, my_referent), Type::Mut(theirs, their_referent)) => {
                Self::same_lifetime(mine, theirs) && self.same_type(my_referent, their_referent)
            }
            _ => mine == theirs,
        }
    }

    fn same_types(&self, mine: &[Type], theirs: &[Type]) -> bool {
        mine.len() == theirs.len()
            && mine
                .iter()
                .zip(theirs)
                .all(|(mine, theirs)| self.same_type(mine, theirs))
    }

    fn same_lifetime(mine: &Lifetime, theirs: &Lifetime) -> bool {
        matches!(
            (mine, theirs),
            (Lifetime::Elided, Lifetime::Elided) | (Lifetime::Written(_), Lifetime::Written(_))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::InlineScheme;
    use crate::types::ty::{GenericId, GenericParam, Scheme, Type};
    use std::rc::Rc;

    struct Schemes;

    impl Schemes {
        fn identity(generic: usize) -> Scheme {
            let id = GenericId::new(generic);
            Scheme {
                lifetimes: Vec::new(),
                params: vec![GenericParam {
                    id,
                    name: "a".into(),
                    bounds: Vec::new(),
                }],
                body: Type::Function(
                    crate::core::ClosureKind::Fn,
                    vec![Type::Generic(id)],
                    Box::new(Type::Generic(id)),
                ),
            }
        }
    }

    #[test]
    fn a_scheme_with_renamed_generics_keeps_one_scheme() {
        let widened =
            InlineScheme::One(Rc::new(Schemes::identity(0))).widened(&Schemes::identity(7));
        assert!(widened.single().is_some());
    }

    #[test]
    fn a_scheme_of_another_shape_makes_several() {
        let mut other = Schemes::identity(7);
        other.body = Type::unit();
        let widened = InlineScheme::One(Rc::new(Schemes::identity(0))).widened(&other);
        assert!(widened.single().is_none());
        assert!(
            InlineScheme::Several
                .widened(&Schemes::identity(0))
                .single()
                .is_none()
        );
    }
}
