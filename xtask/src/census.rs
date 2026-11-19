use std::path::Path;

use crate::failure::TaskFailure;
use crate::ratchet::Measurement;

pub trait Census {
    fn measure(&self, root: &Path) -> Result<Vec<Measurement>, TaskFailure>;
}
