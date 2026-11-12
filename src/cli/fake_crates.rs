use std::path::Path;

use super::entry_span::EntrySpan;
use super::project_crates::ProjectCrates;
use crate::project::CrateDependency;

pub struct FakeCrates;

impl FakeCrates {
    pub const PROJECT: &str = "geometry";
    const CRATE: &str = "answer";
    const CRATE_FIXTURE: &str = "tests/fixtures/crates/answer";

    pub fn with_a_path_crate(root: &Path) -> ProjectCrates {
        ProjectCrates::new(
            root.to_path_buf(),
            Self::PROJECT.to_string(),
            vec![Self::path_crate()],
            vec![],
        )
    }

    fn path_crate() -> CrateDependency {
        CrateDependency {
            name: Self::CRATE.to_string(),
            name_span: EntrySpan::start(),
            version: None,
            features: vec![],
            path: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join(Self::CRATE_FIXTURE)),
        }
    }
}
