pub struct AllocatorShare;

impl AllocatorShare {
    pub const CALLEE_PREFIXES: [&'static str; 2] = ["__rustc::__rust_", "alloc::raw_vec::"];
    const SUFFIX: &'static str = " allocator";

    pub fn is_allocator_call(callee: &str) -> bool {
        Self::CALLEE_PREFIXES
            .iter()
            .any(|prefix| callee.starts_with(prefix))
    }

    pub fn row_of(stage: &str) -> String {
        format!("{stage}{}", Self::SUFFIX)
    }

    pub fn is_row(stage: &str) -> bool {
        Self::stage_of(stage).is_some()
    }

    pub fn stage_of(row: &str) -> Option<&str> {
        row.strip_suffix(Self::SUFFIX)
    }
}

#[cfg(test)]
mod tests {
    use super::AllocatorShare;

    #[test]
    fn names_a_stage_s_allocator_row_by_its_suffix() {
        assert_eq!(AllocatorShare::row_of("build"), "build allocator");
        assert!(AllocatorShare::is_row("build allocator"));
        assert!(!AllocatorShare::is_row("build"));
    }

    #[test]
    fn reads_the_stage_of_an_allocator_row() {
        assert_eq!(AllocatorShare::stage_of("build allocator"), Some("build"));
        assert_eq!(AllocatorShare::stage_of("build"), None);
    }

    #[test]
    fn counts_the_allocator_s_entry_points_and_a_vec_s_growth_as_allocator_calls() {
        assert!(AllocatorShare::is_allocator_call(
            "__rustc::__rust_alloc_zeroed"
        ));
        assert!(AllocatorShare::is_allocator_call(
            "alloc::raw_vec::RawVecInner<A>::reserve::do_reserve_and_handle"
        ));
        assert!(!AllocatorShare::is_allocator_call(
            "compiler::build::token_alignment::RustTokens::scan"
        ));
    }
}
