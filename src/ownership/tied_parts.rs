use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::{GenericId, Lifetime, Type};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum TypeSource {
    Lifetime(BindingId),
    Generic(GenericId),
}

pub struct TiedParts {
    result: FxHashSet<TypeSource>,
}

impl TiedParts {
    pub fn of_result(result: &Type) -> Self {
        Self {
            result: Self::sources(result),
        }
    }

    pub fn ties_only_part_of(&self, parameter: &Type) -> bool {
        let sources = Self::sources(parameter);
        sources.len() > 1 && !sources.is_subset(&self.result)
    }

    fn sources(ty: &Type) -> FxHashSet<TypeSource> {
        let mut sources = FxHashSet::default();
        Self::collect(ty, &mut sources);
        sources
    }

    fn collect(ty: &Type, sources: &mut FxHashSet<TypeSource>) {
        match ty {
            Type::Ref(Lifetime::Written(lifetime), _)
            | Type::Mut(Lifetime::Written(lifetime), _) => {
                sources.insert(TypeSource::Lifetime(*lifetime));
            }
            Type::Generic(generic) => {
                sources.insert(TypeSource::Generic(*generic));
            }
            _ => {}
        }
        for part in ty.parts() {
            Self::collect(part, sources);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TiedParts;
    use crate::core::BindingId;
    use crate::types::{Lifetime, Type};

    struct TiesFixture;

    impl TiesFixture {
        const PAIR: u32 = 0;

        fn borrow(lifetime: u32) -> Type {
            let written = Lifetime::Written(BindingId::from_index(lifetime + 1));
            Type::Ref(written, Box::new(Type::str_ref()))
        }

        fn pair(first: Type, second: Type) -> Type {
            Type::Named(BindingId::from_index(Self::PAIR), vec![first, second])
        }
    }

    #[test]
    fn a_result_of_one_part_ties_only_that_part() {
        let parameter = TiesFixture::pair(TiesFixture::borrow(0), TiesFixture::borrow(1));
        let tied = TiedParts::of_result(&TiesFixture::borrow(1));
        assert!(tied.ties_only_part_of(&parameter));
    }

    #[test]
    fn a_result_of_every_part_ties_the_whole_parameter() {
        let parameter = TiesFixture::pair(TiesFixture::borrow(0), TiesFixture::borrow(1));
        let tied = TiedParts::of_result(&parameter);
        assert!(!tied.ties_only_part_of(&parameter));
    }

    #[test]
    fn a_parameter_of_one_lifetime_is_tied_whole() {
        let parameter = TiesFixture::pair(TiesFixture::borrow(0), TiesFixture::borrow(0));
        let tied = TiedParts::of_result(&TiesFixture::borrow(1));
        assert!(!tied.ties_only_part_of(&parameter));
    }
}
