use crate::heavy_run::{FilterRule, TestArguments};

pub struct AnyFilter;

pub type StageFilters = TestArguments<AnyFilter>;

impl FilterRule for AnyFilter {
    fn narrows(filters: &[&str], _exact: bool) -> bool {
        !filters.is_empty()
    }
}
