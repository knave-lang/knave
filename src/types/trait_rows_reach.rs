use super::ty::{Bound, Scheme, Type};
use crate::core::BindingId;

pub struct Implementor {
    pub scheme: Scheme,
    pub argument: Option<Type>,
}

pub struct TraitRowsReach(Lowering);

enum Lowering {
    Unreached,
    Lowered(LoweredRows),
}

impl TraitRowsReach {
    const LEFT_OUT: &'static str = "the trait reach left out a trait the program uses";

    pub fn unreached() -> Self {
        Self(Lowering::Unreached)
    }

    pub fn lowered(rows: LoweredRows) -> Self {
        Self(Lowering::Lowered(rows))
    }

    pub fn get(&self) -> Option<&LoweredRows> {
        match &self.0 {
            Lowering::Unreached => None,
            Lowering::Lowered(rows) => Some(rows),
        }
    }

    pub fn reached(&self) -> &LoweredRows {
        self.get().expect(Self::LEFT_OUT)
    }

    pub fn reached_mut(&mut self) -> &mut LoweredRows {
        match &mut self.0 {
            Lowering::Unreached => panic!("{}", Self::LEFT_OUT),
            Lowering::Lowered(rows) => rows,
        }
    }

    pub fn assert_reached(&self) {
        self.reached();
    }
}

pub struct LoweredRows {
    implementors: Vec<Implementor>,
    supertraits: Vec<Bound>,
}

impl LoweredRows {
    pub fn new(implementors: Vec<Implementor>, supertraits: Vec<Bound>) -> Self {
        Self {
            implementors,
            supertraits,
        }
    }

    pub fn implementors(&self) -> &[Implementor] {
        &self.implementors
    }

    pub fn add(&mut self, implementor: Implementor) {
        self.implementors.push(implementor);
    }

    pub fn remove_struct(&mut self, struct_binding: BindingId) {
        self.implementors.retain(|implementor| {
            !matches!(implementor.scheme.body, Type::Named(named, _) if named == struct_binding)
        });
    }

    pub fn set_scheme(&mut self, body: &Type, scheme: Scheme) {
        if let Some(implementor) = self
            .implementors
            .iter_mut()
            .find(|implementor| implementor.scheme.body == *body)
        {
            implementor.scheme = scheme;
        }
    }

    pub fn supertraits(&self) -> &[Bound] {
        &self.supertraits
    }

    pub fn is_open(&self) -> bool {
        self.implementors
            .iter()
            .any(|implementor| !implementor.scheme.params.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::{LoweredRows, TraitRowsReach};

    struct RowsFixture;

    impl RowsFixture {
        fn lowered() -> TraitRowsReach {
            TraitRowsReach::lowered(LoweredRows::new(Vec::new(), Vec::new()))
        }
    }

    #[test]
    fn lowered_rows_pass_the_reach_assertion() {
        let mut rows = RowsFixture::lowered();
        rows.assert_reached();
        assert!(rows.get().is_some());
        assert!(rows.reached_mut().implementors().is_empty());
    }

    #[test]
    fn unreached_rows_have_nothing_to_get() {
        assert!(TraitRowsReach::unreached().get().is_none());
    }

    #[test]
    #[should_panic(expected = "the trait reach left out a trait the program uses")]
    fn unreached_rows_fail_the_reach_assertion() {
        TraitRowsReach::unreached().assert_reached();
    }

    #[test]
    #[should_panic(expected = "the trait reach left out a trait the program uses")]
    fn unreached_rows_refuse_a_write() {
        TraitRowsReach::unreached().reached_mut();
    }
}
