use super::loan_place::LoanPlace;
use super::place_access::{AccessKind, PlaceAccess};
use crate::core::{BindingArena, ReferenceKind};
use crate::source::Span;

pub struct Lending {
    place: LoanPlace,
    kind: ReferenceKind,
    span: Span,
}

impl Lending {
    pub fn new(place: LoanPlace, kind: ReferenceKind, span: Span) -> Self {
        Self { place, kind, span }
    }

    pub fn access(&self) -> PlaceAccess {
        let kind = match self.kind {
            ReferenceKind::Shared => AccessKind::Borrow,
            ReferenceKind::Mutable => AccessKind::Change,
        };
        PlaceAccess::new(self.place.clone(), kind, self.span)
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn written(&self, bindings: &BindingArena) -> String {
        let kind = match self.kind {
            ReferenceKind::Shared => "",
            ReferenceKind::Mutable => "mut ",
        };
        format!("{kind}`{}`", self.place.written(bindings))
    }
}

#[cfg(test)]
mod tests {
    use super::Lending;
    use crate::core::ReferenceKind;
    use crate::ownership::lambda_source::LambdaSource;
    use crate::ownership::loans::loan_place::LoanPlace;

    struct LendingFixture {
        source: LambdaSource,
    }

    impl LendingFixture {
        fn new() -> Self {
            Self {
                source: LambdaSource::new(
                    "(defn main ()\n  (local v (vec 1))\n  (local w v)\n  (println (len w)))",
                ),
            }
        }

        fn lending(&self, kind: ReferenceKind) -> Lending {
            let moved = self.source.bound("w");
            let place = LoanPlace::of(moved).expect("a local is a place");
            Lending::new(place, kind, moved.span)
        }

        fn written(&self, kind: ReferenceKind) -> (String, String) {
            let bindings = &self.source.program().bindings;
            let lending = self.lending(kind);
            (
                lending.written(bindings),
                lending.access().written(bindings),
            )
        }
    }

    #[test]
    fn a_shared_loan_borrows_its_place_and_a_mutable_one_changes_it() {
        let fixture = LendingFixture::new();
        assert_eq!(
            fixture.written(ReferenceKind::Shared),
            ("`v`".to_string(), "borrow `v`".to_string())
        );
        assert_eq!(
            fixture.written(ReferenceKind::Mutable),
            ("mut `v`".to_string(), "change `v`".to_string())
        );
    }
}
