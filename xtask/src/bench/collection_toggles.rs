use crate::failure::TaskFailure;

pub struct CollectionToggles;

impl CollectionToggles {
    pub const THREAD_ENTRY: &'static str = "std::sys::backtrace::__rust_begin_short_backtrace";
    pub const RUST_ALLOCATOR: &'static str = "__rustc::__rust_*";
    const OPTION: &'static str = "--toggle-collect";

    pub fn options() -> [String; 2] {
        [Self::THREAD_ENTRY, Self::RUST_ALLOCATOR].map(Self::option_of)
    }

    pub fn nothing_collected() -> TaskFailure {
        TaskFailure::new(format!(
            "callgrind collected nothing under `{}`: the toolchain may have renamed or inlined \
             that function",
            Self::option_of(Self::THREAD_ENTRY)
        ))
    }

    fn option_of(function: &str) -> String {
        format!("{}={function}", Self::OPTION)
    }
}
