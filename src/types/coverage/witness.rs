use super::constructor::{Constructor, Tag};
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
            Tag::Variant { owner, index } => Witness::Variant {
                owner,
                index,
                arguments: arguments.to_vec(),
            },
            Tag::Struct(owner) => Witness::Struct {
                owner,
                arguments: arguments.to_vec(),
            },
            Tag::Bool(value) => Witness::Bool(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Witness;
    use crate::core::BindingId;
    use crate::types::coverage::constructor::{Constructor, Tag};

    struct WitnessFixture {
        owner: BindingId,
    }

    impl WitnessFixture {
        fn new() -> Self {
            Self {
                owner: BindingId::from_index(3),
            }
        }

        fn built_by(tag: Tag) -> Witness {
            Witness::built_by(&Constructor::new(tag, Vec::new()), &[Witness::Wild])
        }
    }

    #[test]
    fn a_variant_constructor_builds_a_variant_witness_with_its_arguments() {
        let fixture = WitnessFixture::new();
        let witness = WitnessFixture::built_by(Tag::Variant {
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
        let witness = WitnessFixture::built_by(Tag::Struct(fixture.owner));
        assert!(matches!(
            witness,
            Witness::Struct { owner, arguments } if owner == fixture.owner && arguments.len() == 1
        ));
    }

    #[test]
    fn a_bool_constructor_builds_its_bool() {
        assert!(matches!(
            WitnessFixture::built_by(Tag::Bool(false)),
            Witness::Bool(false)
        ));
    }
}
