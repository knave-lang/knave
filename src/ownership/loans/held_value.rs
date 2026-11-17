use super::loan_depth::LoanDepth;
use crate::core::BindingId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeldValue {
    binding: BindingId,
    depth: isize,
}

impl HeldValue {
    pub fn of(binding: BindingId) -> Self {
        Self { binding, depth: 0 }
    }

    pub fn binding(&self) -> BindingId {
        self.binding
    }

    pub fn within(self) -> Self {
        Self {
            depth: self.depth + 1,
            ..self
        }
    }

    pub fn referent(self) -> Self {
        Self {
            depth: self.depth - 1,
            ..self
        }
    }

    pub fn placed(&self, layer: LoanDepth) -> Option<LoanDepth> {
        layer.shifted(self.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::HeldValue;
    use crate::core::BindingId;
    use crate::ownership::loans::loan_depth::LoanDepth;

    #[test]
    fn a_value_held_within_places_each_layer_one_deeper_and_its_referent_one_higher() {
        let value = HeldValue::of(BindingId::from_index(1));
        assert_eq!(
            value.within().placed(LoanDepth::TOP),
            Some(LoanDepth::at(1))
        );
        assert_eq!(
            value.referent().placed(LoanDepth::at(1)),
            Some(LoanDepth::TOP)
        );
        assert_eq!(value.referent().placed(LoanDepth::TOP), None);
    }
}
