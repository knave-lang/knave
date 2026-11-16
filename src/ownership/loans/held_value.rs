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

    pub fn placed(&self, layer: usize) -> Option<usize> {
        layer.checked_add_signed(self.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::HeldValue;
    use crate::core::BindingId;

    #[test]
    fn a_value_held_within_places_each_layer_one_deeper_and_its_referent_one_higher() {
        let value = HeldValue::of(BindingId::from_index(1));
        assert_eq!(value.within().placed(0), Some(1));
        assert_eq!(value.referent().placed(1), Some(0));
        assert_eq!(value.referent().placed(0), None);
    }
}
