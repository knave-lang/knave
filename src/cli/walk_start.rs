use std::path::Path;

use super::checkout_bounds::CheckoutBounds;

pub struct WalkStart<'s> {
    pub root: &'s Path,
    pub checkout: Option<CheckoutBounds>,
}
