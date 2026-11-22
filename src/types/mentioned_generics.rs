use super::ty::{GenericId, Type};

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

    fn collect(&mut self, value: &Type) {
        match value {
            Type::Generic(id) if !self.ids.contains(id) => self.ids.push(*id),
            Type::Generic(_) => {}
            _ => value
                .parts()
                .into_iter()
                .for_each(|part| self.collect(part)),
        }
    }

    pub(super) fn into_ids(self) -> Vec<GenericId> {
        self.ids
    }
}

#[cfg(test)]
mod tests {
    use super::MentionedGenerics;
    use crate::core::{ClosureKind, PrimitiveType};
    use crate::types::{GenericId, Type};

    struct Generics;

    impl Generics {
        fn generic(index: usize) -> Type {
            Type::Generic(GenericId::new(index))
        }

        fn function(parameters: Vec<Type>, result: Type) -> Type {
            Type::Function(ClosureKind::Fn, parameters, Box::new(result))
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
}
