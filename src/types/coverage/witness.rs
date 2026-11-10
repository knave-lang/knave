use super::constructor::Constructor;
use super::constructor_tag::ConstructorTag;
use crate::core::BindingId;

#[derive(Clone)]
pub(super) enum Witness {
    Wild,
    Variant {
        owner: BindingId,
        index: usize,
        arguments: Vec<Witness>,
    },
    Struct {
        owner: BindingId,
        arguments: Vec<Witness>,
    },
    Bool(bool),
}

impl Witness {
    pub(super) fn built_by(constructor: &Constructor, arguments: &[Witness]) -> Self {
        match constructor.tag() {
            ConstructorTag::Variant { owner, index } => Witness::Variant {
                owner,
                index,
                arguments: arguments.to_vec(),
            },
            ConstructorTag::Struct(owner) => Witness::Struct {
                owner,
                arguments: arguments.to_vec(),
            },
            ConstructorTag::Bool(value) => Witness::Bool(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Witness;
    use crate::core::BindingId;
    use crate::types::coverage::constructor::Constructor;
    use crate::types::coverage::constructor_tag::ConstructorTag;

    struct WitnessFixture {
        owner: BindingId,
    }

    impl WitnessFixture {
        fn new() -> Self {
            Self {
                owner: BindingId::from_index(3),
            }
        }

        fn built_by(tag: ConstructorTag) -> Witness {
            Witness::built_by(&Constructor::new(tag, Vec::new()), &[Witness::Wild])
        }
    }

    #[test]
    fn a_variant_constructor_builds_a_variant_witness_with_its_arguments() {
        let fixture = WitnessFixture::new();
        let witness = WitnessFixture::built_by(ConstructorTag::Variant {
            owner: fixture.owner,
            index: 2,
        });
        assert!(matches!(
            witness,
            Witness::Variant { owner, index: 2, arguments } if owner == fixture.owner && arguments.len() == 1
        ));
    }

    #[test]
    fn a_struct_constructor_builds_a_struct_witness_with_its_arguments() {
        let fixture = WitnessFixture::new();
        let witness = WitnessFixture::built_by(ConstructorTag::Struct(fixture.owner));
        assert!(matches!(
            witness,
            Witness::Struct { owner, arguments } if owner == fixture.owner && arguments.len() == 1
        ));
    }

    #[test]
    fn a_bool_constructor_builds_its_bool() {
        assert!(matches!(
            WitnessFixture::built_by(ConstructorTag::Bool(false)),
            Witness::Bool(false)
        ));
    }
}
