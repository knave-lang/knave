use super::ty::{GenericId, GenericParam, Type};

#[derive(Default)]
pub(super) struct MentionedGenerics {
    ids: Vec<GenericId>,
}

impl MentionedGenerics {
    pub(super) fn of(value: &Type) -> Self {
        let mut mentioned = Self::default();
        mentioned.collect(value);
        mentioned
    }

    pub(super) fn collect(&mut self, value: &Type) {
        match value {
            Type::Generic(id) if !self.ids.contains(id) => self.ids.push(*id),
            Type::Generic(_) => {}
            _ => value
                .parts()
                .into_iter()
                .for_each(|part| self.collect(part)),
        }
    }

    pub(super) fn with_bound_arguments<'p>(
        mut self,
        param_of: impl Fn(GenericId) -> Option<&'p GenericParam>,
    ) -> Self {
        let mut position = 0;
        while let Some(id) = self.get(position) {
            let bounds = param_of(id).into_iter().flat_map(|param| &param.bounds);
            for argument in bounds.filter_map(|bound| bound.argument.as_ref()) {
                self.collect(argument);
            }
            position += 1;
        }
        self
    }

    pub(super) fn get(&self, position: usize) -> Option<GenericId> {
        self.ids.get(position).copied()
    }

    pub(super) fn into_ids(self) -> Vec<GenericId> {
        self.ids
    }
}

#[cfg(test)]
mod tests {
    use super::MentionedGenerics;
    use crate::core::{BindingId, ClosureKind, PrimitiveType};
    use crate::types::{Bound, GenericId, GenericParam, Type};

    struct Generics;

    impl Generics {
        fn generic(index: usize) -> Type {
            Type::Generic(GenericId::new(index))
        }

        fn function(parameters: Vec<Type>, result: Type) -> Type {
            Type::Function(ClosureKind::Fn, parameters, Box::new(result))
        }

        fn bounded(index: usize, argument: Type) -> GenericParam {
            GenericParam {
                id: GenericId::new(index),
                name: "a".into(),
                bounds: vec![Bound::with_argument(BindingId::from_index(0), argument)],
            }
        }
    }

    #[test]
    fn collects_each_generic_once_in_order_of_mention() {
        let function = Generics::function(
            vec![Generics::generic(1), Type::Primitive(PrimitiveType::I64)],
            Generics::function(vec![Generics::generic(0)], Generics::generic(1)),
        );
        let ids = MentionedGenerics::of(&function).into_ids();
        assert_eq!(ids, vec![GenericId::new(1), GenericId::new(0)]);
    }

    #[test]
    fn reads_a_generic_by_its_position() {
        let mentioned = MentionedGenerics::of(&Generics::generic(2));
        assert_eq!(mentioned.get(0), Some(GenericId::new(2)));
        assert_eq!(mentioned.get(1), None);
    }

    #[test]
    fn follows_the_arguments_of_each_bound() {
        let owner = Generics::bounded(0, Generics::generic(1));
        let mentioned = MentionedGenerics::of(&Generics::generic(0))
            .with_bound_arguments(|id| (id == owner.id).then_some(&owner));
        assert_eq!(
            mentioned.into_ids(),
            vec![GenericId::new(0), GenericId::new(1)]
        );
    }
}
