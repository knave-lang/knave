use super::definition_kind::DefinitionKind;
use crate::core::{BindingId, CoreModule, ItemKind};

pub(super) struct DeriveTarget {
    pub(super) binding: BindingId,
    pub(super) arity: usize,
    pub(super) kind: DefinitionKind,
}

impl DeriveTarget {
    pub(super) fn all(modules: &[CoreModule]) -> Vec<Self> {
        modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::Struct(binding, params, _) => {
                    Some(Self::new(*binding, params.len(), DefinitionKind::Struct))
                }
                ItemKind::Enum(binding, params, ..) => {
                    Some(Self::new(*binding, params.len(), DefinitionKind::Enum))
                }
                _ => None,
            })
            .collect()
    }

    fn new(binding: BindingId, arity: usize, kind: DefinitionKind) -> Self {
        Self {
            binding,
            arity,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DeriveTarget;
    use crate::types::declarations_fixture::DeclarationsFixture;
    use crate::types::definition_kind::DefinitionKind;

    const SOURCE: &str = "
        (struct (duet a b) (left a) (right b))
        (enum signal on off)
        (defn main () (do))";

    struct TargetFixture {
        declarations: DeclarationsFixture,
    }

    impl TargetFixture {
        fn new() -> Self {
            Self {
                declarations: DeclarationsFixture::new(SOURCE),
            }
        }

        fn target(&self, name: &str) -> DeriveTarget {
            let binding = self.declarations.type_named(name);
            DeriveTarget::all(self.declarations.modules())
                .into_iter()
                .find(|target| target.binding == binding)
                .expect("the type is a derive target")
        }
    }

    #[test]
    fn a_struct_is_a_target_with_its_arity() {
        let duet = TargetFixture::new().target("duet");
        assert_eq!(duet.arity, 2);
        assert!(matches!(duet.kind, DefinitionKind::Struct));
    }

    #[test]
    fn an_enum_is_a_target_of_the_enum_kind() {
        let signal = TargetFixture::new().target("signal");
        assert_eq!(signal.arity, 0);
        assert!(matches!(signal.kind, DefinitionKind::Enum));
    }
}
