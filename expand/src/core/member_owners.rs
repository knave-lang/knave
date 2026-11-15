use super::{BindingId, ItemKind};

pub struct MemberOwners<'i> {
    kind: &'i ItemKind,
}

impl<'i> MemberOwners<'i> {
    pub fn of(kind: &'i ItemKind) -> Self {
        Self { kind }
    }

    pub fn visit(&self, visit: &mut dyn FnMut(BindingId, BindingId)) {
        match self.kind {
            ItemKind::Trait(owner, _, _, _, _, methods) => {
                methods
                    .iter()
                    .for_each(|method| visit(method.binding, *owner));
            }
            ItemKind::RustTrait(declaration) => {
                let methods = declaration.methods.iter().map(|method| method.binding);
                let associated = declaration.associated.iter().map(|value| value.binding);
                methods
                    .chain(associated)
                    .for_each(|member| visit(member, declaration.binding));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::MemberOwners;
    use crate::core::ModulePath;
    use crate::expand::Expander;
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct MemberOwnersFixture;

    impl MemberOwnersFixture {
        const MODULE: &str = "shapes";

        fn owned_by(text: &str) -> Vec<(String, String)> {
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::MODULE, text);
            let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
            let program = Expander::new(&mut sources, module_sources)
                .expand(ModulePath::new(Self::MODULE), entry)
                .unwrap_or_else(|_| panic!("the program expands"));
            let name = |binding| program.bindings.get(binding).name().to_string();
            let mut owned = Vec::new();
            for item in &program.entry_module().items {
                MemberOwners::of(&item.kind)
                    .visit(&mut |member, owner| owned.push((name(member), name(owner))));
            }
            owned
        }

        fn pair(member: &str, owner: &str) -> (String, String) {
            (member.to_string(), owner.to_string())
        }
    }

    #[test]
    fn a_trait_owns_its_methods() {
        let owned = MemberOwnersFixture::owned_by("(trait sized (size (fn ((ref self)) i64)))");
        assert_eq!(owned, vec![MemberOwnersFixture::pair("size", "sized")]);
    }

    #[test]
    fn a_rust_trait_owns_its_methods() {
        let owned = MemberOwnersFixture::owned_by(
            "(rust-trait shown \"std::fmt::Display\" (method shown-width (fn ((ref self)) i64)) i64)",
        );
        assert_eq!(
            owned,
            vec![MemberOwnersFixture::pair("shown-width", "shown")]
        );
    }

    #[test]
    fn a_rust_trait_owns_its_associated_types() {
        let owned = MemberOwnersFixture::owned_by(
            "(rust-trait addable \"std::ops::Add\"
              (associated (output left) \"Output\")
              (where () (with i64 i64) (output i64)))",
        );
        assert_eq!(owned, vec![MemberOwnersFixture::pair("output", "addable")]);
    }

    #[test]
    fn a_struct_owns_no_members() {
        let owned = MemberOwnersFixture::owned_by("(defstruct point :x i64)");
        assert!(owned.is_empty(), "{owned:?}");
    }
}
