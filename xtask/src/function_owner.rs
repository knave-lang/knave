use syn::{Ident, ItemImpl, ItemTrait, Type};

#[derive(Default)]
pub struct FunctionOwner {
    name: Option<String>,
}

impl FunctionOwner {
    pub fn of_impl(implementation: &ItemImpl) -> Self {
        let name = match implementation.self_ty.as_ref() {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string()),
            _ => None,
        };
        Self { name }
    }

    pub fn of_trait(definition: &ItemTrait) -> Self {
        Self {
            name: Some(definition.ident.to_string()),
        }
    }

    pub fn qualify(&self, function: &Ident) -> String {
        match &self.name {
            Some(owner) => format!("{owner}::{function}"),
            None => function.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::{Ident, ItemImpl, ItemTrait};

    use super::FunctionOwner;

    struct Owner;

    impl Owner {
        fn of_impl(text: &str) -> FunctionOwner {
            let implementation: ItemImpl = syn::parse_str(text).expect("the impl parses");
            FunctionOwner::of_impl(&implementation)
        }

        fn function() -> Ident {
            syn::parse_str("area").expect("the name parses")
        }
    }

    #[test]
    fn an_impl_names_its_type_by_the_last_path_segment() {
        let owner = Owner::of_impl("impl shapes::Square { }");
        assert_eq!(owner.qualify(&Owner::function()), "Square::area");
    }

    #[test]
    fn an_impl_for_a_type_without_a_path_has_no_owner() {
        let owner = Owner::of_impl("impl Shape for &Square { }");
        assert_eq!(owner.qualify(&Owner::function()), "area");
    }

    #[test]
    fn a_trait_owns_its_methods() {
        let definition: ItemTrait = syn::parse_str("trait Shape { }").expect("the trait parses");
        let owner = FunctionOwner::of_trait(&definition);
        assert_eq!(owner.qualify(&Owner::function()), "Shape::area");
    }
}
