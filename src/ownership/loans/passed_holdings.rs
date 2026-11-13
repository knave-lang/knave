use super::holdings::Holdings;

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
}
