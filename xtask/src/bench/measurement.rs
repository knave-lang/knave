use std::collections::BTreeMap;

use super::collection_toggles::CollectionToggles;
use super::gated_counters::GatedCounters;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measurement {
    stages: BTreeMap<String, u64>,
    total: u64,
}

impl Measurement {
    pub const OTHER: &'static str = "other";
    pub const TOTAL: &'static str = "total";

    pub fn new(stages: BTreeMap<String, u64>, total: u64) -> Self {
        Self { stages, total }
    }

    pub fn method() -> String {
        format!(
            "Callgrind counts each corpus program under the compiler's `emit` command, which reads, expands, checks, \
             emits and formats the program but never runs `rustc`. \
             On each thread, callgrind collects only inside the thread's entry function, \
             `{thread_entry}`, and nothing inside Rust's allocator functions, `{rust_allocator}`. \
             The one exception is the allocator work that std's thread setup and teardown start \
             outside every entry function. It counts in `{other}` and is nearly the same in every \
             run.\n\n\
             A stage is the inclusive cost of the calls the command-line module makes into one \
             compiler module. `{other}` holds the rest: argument parsing, \
             reading the entry file, printing, dropping each stage's result, and the threads \
             that read `rustfmt`'s output. \
             `rustfmt` runs as a child process that callgrind does not trace, so the `build` stage \
             counts only the compiler's side of formatting.\n\n\
             The compiler hashes with a fixed seed, so its hash maps lay out the same way in \
             every run. Identical runs still differ by a few hundred instructions at most.\n\n\
             The compiler runs with glibc's fastbins turned off \
             (`GLIBC_TUNABLES=glibc.malloc.mxfast=0`), so glibc merges freed memory when it is \
             freed, and no stage pays to merge memory that an earlier stage freed.\n\n\
             The bench then builds each compiler again with the `counters` feature and runs it \
             once on each corpus program, without valgrind, as `--counters emit` on the \
             program's file name from the program's directory, with only `PATH` set. Each counter \
             of that run is recorded. The check fails when a program's {gated} rises more \
             than {tolerance}% above the base commit, unless a commit of \
             the pull request names the program in a `Counters-Accepted:` trailer.\n",
            other = Self::OTHER,
            thread_entry = CollectionToggles::THREAD_ENTRY,
            rust_allocator = CollectionToggles::RUST_ALLOCATOR,
            gated = GatedCounters::listed(),
            tolerance = GatedCounters::TOLERANCE_PERCENT,
        )
    }

    pub fn modules(&self) -> impl Iterator<Item = &str> {
        self.stages.keys().map(String::as_str)
    }

    pub fn rows(&self) -> impl Iterator<Item = (&str, u64)> {
        let staged: u64 = self.stages.values().sum();
        self.stages
            .iter()
            .map(|(stage, instructions)| (stage.as_str(), *instructions))
            .chain([
                (Self::OTHER, self.total.saturating_sub(staged)),
                (Self::TOTAL, self.total),
            ])
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn instructions_of(&self, stage: &str) -> Option<u64> {
        self.rows()
            .find(|(name, _)| *name == stage)
            .map(|(_, instructions)| instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_both_toggled_functions_in_the_method() {
        let method = Measurement::method();
        assert!(method.contains(CollectionToggles::THREAD_ENTRY), "{method}");
        assert!(
            method.contains(CollectionToggles::RUST_ALLOCATOR),
            "{method}"
        );
    }

    #[test]
    fn charges_the_unstaged_remainder_to_other() {
        let stages = BTreeMap::from([("emit".to_string(), 30)]);
        let measurement = Measurement::new(stages, 100);
        assert_eq!(measurement.instructions_of(Measurement::OTHER), Some(70));
        assert_eq!(measurement.instructions_of("read"), None);
    }
}
