use rustc_hash::FxHashSet;

use crate::core::BindingId;
use crate::types::{GenericId, Lifetime, Type, TypeLifetimes};

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
        let lifetimes =
            TypeLifetimes::of(ty)
                .all()
                .into_iter()
                .filter_map(|lifetime| match lifetime {
                    Lifetime::Written(binding) => Some(TypeSource::Lifetime(binding)),
                    Lifetime::Elided => None,
                });
        let mut sources: FxHashSet<TypeSource> = lifetimes.collect();
        Self::collect_generics(ty, &mut sources);
        sources
    }

    fn collect_generics(ty: &Type, sources: &mut FxHashSet<TypeSource>) {
        match ty {
            Type::Function(..) => {}
            Type::Generic(generic) => {
                sources.insert(TypeSource::Generic(*generic));
            }
            _ => ty
                .parts()
                .into_iter()
                .for_each(|part| Self::collect_generics(part, sources)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TiedParts;
    use crate::core::BindingId;
    use crate::types::{ClosureKind, GenericId, Lifetime, Type};

    struct TiesFixture;

    impl TiesFixture {
        const PAIR: u32 = 0;

        fn borrow(lifetime: u32) -> Type {
            let written = Lifetime::Written(BindingId::from_index(lifetime + 1));
            Type::Ref(written, Box::new(Type::str_ref()))
        }

        fn generic(index: usize) -> Type {
            Type::Generic(GenericId::new(index))
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
    fn a_lifetime_inside_a_function_type_is_no_source() {
        let function = Type::Function(
            ClosureKind::Fn,
            vec![TiesFixture::borrow(0)],
            Box::new(Type::unit()),
        );
        let parameter = TiesFixture::pair(function, TiesFixture::borrow(1));
        let tied = TiedParts::of_result(&TiesFixture::borrow(1));
        assert!(!tied.ties_only_part_of(&parameter));
    }

    #[test]
    fn a_parameter_of_one_lifetime_is_tied_whole() {
        let parameter = TiesFixture::pair(TiesFixture::borrow(0), TiesFixture::borrow(0));
        let tied = TiedParts::of_result(&TiesFixture::borrow(1));
        assert!(!tied.ties_only_part_of(&parameter));
    }

    #[test]
    fn an_elided_lifetime_is_no_source() {
        let parameter = TiesFixture::pair(Type::str_ref(), TiesFixture::generic(0));
        let tied = TiedParts::of_result(&TiesFixture::generic(0));
        assert!(!tied.ties_only_part_of(&parameter));
    }

    #[test]
    fn a_type_variable_inside_a_function_type_is_no_source() {
        let function = Type::Function(
            ClosureKind::Fn,
            vec![TiesFixture::generic(0)],
            Box::new(Type::unit()),
        );
        let parameter = TiesFixture::pair(function, TiesFixture::generic(1));
        let tied = TiedParts::of_result(&TiesFixture::generic(1));
        assert!(!tied.ties_only_part_of(&parameter));
    }
}
