use super::holdings::Holdings;
use crate::ownership::stored_layer::StoredLayer;

pub struct PassedHoldings {
    held: Holdings,
    lent: Holdings,
}

impl PassedHoldings {
    pub fn kept(held: Holdings) -> Self {
        Self {
            held,
            lent: Holdings::default(),
        }
    }

    pub fn lending(held: Holdings, lent: Holdings) -> Self {
        Self { held, lent }
    }

    pub fn held(&self) -> &Holdings {
        &self.held
    }

    pub fn all(&self) -> Holdings {
        self.held.clone().merged(self.lent.clone())
    }

    pub fn stored(&self, layer: StoredLayer) -> Holdings {
        match layer {
            StoredLayer::Whole => self.all(),
            StoredLayer::Referent => self.held.referent(),
        }
    }
}
