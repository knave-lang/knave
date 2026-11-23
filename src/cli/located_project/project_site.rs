use std::path::Path;

use crate::cli::checkout_bounds::CheckoutBounds;

pub enum ProjectSite<'s> {
    Around(&'s Path),
    Library {
        root: &'s Path,
        checkout: Option<CheckoutBounds>,
    },
}
