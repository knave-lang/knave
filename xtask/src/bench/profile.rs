use std::collections::BTreeSet;

use super::call::Call;
use super::call_record::CallRecord;
use super::collection_toggles::CollectionToggles;
use super::measurement::Measurement;
use super::stage_entry::StageEntry;
use crate::failure::TaskFailure;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryPointCheck {
    Enforced,
    Skipped,
}

pub struct ProfileReader<'a> {
    crate_names: &'a BTreeSet<String>,
    entry_point_check: EntryPointCheck,
}

impl<'a> ProfileReader<'a> {
    const ENTRY_MODULE: &'static str = "cli";
    const STAGE_ENTRY_POINTS: [&'static str; 4] = [
        "expand::Expander::expand",
        "check::ProgramChecker::check",
        "emit::RustEmitter::emit",
        "build::formatting::EmittedFormatting::apply",
    ];

    pub fn new(crate_names: &'a BTreeSet<String>, entry_point_check: EntryPointCheck) -> Self {
        Self {
            crate_names,
            entry_point_check,
        }
    }

    pub fn read(&self, profile: &str) -> Result<Measurement, TaskFailure> {
        let mut scan = ProfileScan::new(self);
        profile.lines().for_each(|line| scan.consume(line));
        scan.finish()
    }

    fn missing_entry_point(&self, entered: &BTreeSet<&str>) -> Option<Vec<String>> {
        if self.entry_point_check == EntryPointCheck::Skipped {
            return None;
        }
        Self::STAGE_ENTRY_POINTS
            .iter()
            .map(|entry_point| self.in_each_crate(entry_point))
            .find(|candidates| {
                !candidates
                    .iter()
                    .any(|candidate| entered.contains(candidate.as_str()))
            })
    }

    fn in_each_crate(&self, path: &str) -> Vec<String> {
        self.crate_names
            .iter()
            .map(|crate_name| format!("{crate_name}::{path}"))
            .collect()
    }

    fn stage_entries(&self) -> Vec<StageEntry> {
        Self::STAGE_ENTRY_POINTS
            .iter()
            .flat_map(|entry_point| self.in_each_crate(entry_point))
            .filter_map(|function| {
                let stage = self.module_of(&function)?.to_string();
                Some(StageEntry { stage, function })
            })
            .collect()
    }

    fn stage_of_call(&self, caller: &str, callee: &str) -> Option<String> {
        let callee_module = self.module_of(callee)?;
        let leaves_entry = self.module_of(caller) == Some(Self::ENTRY_MODULE)
            && callee_module != Self::ENTRY_MODULE;
        leaves_entry.then(|| callee_module.to_string())
    }

    fn module_of<'name>(&self, function: &'name str) -> Option<&'name str> {
        let item = function.trim_start_matches('<');
        self.crate_names
            .iter()
            .find_map(|crate_name| Self::module_within(item, crate_name))
    }

    fn module_within<'name>(item: &'name str, crate_name: &str) -> Option<&'name str> {
        let path = item.strip_prefix(crate_name)?.strip_prefix("::")?;
        let length = path
            .find(|character: char| !character.is_alphanumeric() && character != '_')
            .unwrap_or(path.len());
        (length > 0).then(|| &path[..length])
    }
}

enum ProfileLine<'a> {
    Events(&'a str),
    Summary(&'a str),
    Caller(&'a str),
    Callee(&'a str),
    Call,
    Other,
}

impl<'a> ProfileLine<'a> {
    fn classify(line: &'a str) -> Self {
        if let Some(header) = line.strip_prefix("events:") {
            Self::Events(header)
        } else if let Some(counts) = line.strip_prefix("summary:") {
            Self::Summary(counts)
        } else if let Some(name) = line.strip_prefix("fn=") {
            Self::Caller(name)
        } else if let Some(name) = line.strip_prefix("cfn=") {
            Self::Callee(name)
        } else if line.starts_with("calls=") {
            Self::Call
        } else {
            Self::Other
        }
    }
}

struct ProfileScan<'reader, 'text> {
    reader: &'reader ProfileReader<'reader>,
    instruction_column: Option<usize>,
    summary: Option<Vec<u64>>,
    caller: &'text str,
    callee: &'text str,
    call_cost_follows: bool,
    record: CallRecord<'text>,
}

impl<'reader, 'text> ProfileScan<'reader, 'text> {
    const INSTRUCTIONS_EVENT: &'static str = "Ir";

    fn new(reader: &'reader ProfileReader<'reader>) -> Self {
        Self {
            reader,
            instruction_column: None,
            summary: None,
            caller: "",
            callee: "",
            call_cost_follows: false,
            record: CallRecord::default(),
        }
    }

    fn consume(&mut self, line: &'text str) {
        if self.call_cost_follows {
            self.call_cost_follows = false;
            self.record_call(line);
            return;
        }
        match ProfileLine::classify(line) {
            ProfileLine::Events(header) => self.instruction_column = Self::column_of(header),
            ProfileLine::Summary(counts) => self.summary = Some(Self::counts(counts)),
            ProfileLine::Caller(name) => self.caller = name,
            ProfileLine::Callee(name) => self.callee = name,
            ProfileLine::Call => self.call_cost_follows = true,
            ProfileLine::Other => {}
        }
    }

    fn record_call(&mut self, cost_line: &str) {
        let Some(column) = self.instruction_column else {
            return;
        };
        let counts = Self::counts(cost_line.split_once(' ').map_or("", |(_, counts)| counts));
        let cost = Self::count_at(&counts, column);
        let call = Call {
            caller: self.caller,
            callee: self.callee,
            cost,
        };
        let stage = self.reader.stage_of_call(self.caller, self.callee);
        self.record.record(call, stage);
    }

    fn finish(self) -> Result<Measurement, TaskFailure> {
        let column = self
            .instruction_column
            .ok_or(TaskFailure::malformed_profile("events"))?;
        let summary = self
            .summary
            .ok_or(TaskFailure::malformed_profile("summary"))?;
        if self.record.collected_nothing_in_a_thread_entry() {
            return Err(CollectionToggles::nothing_collected());
        }
        if let Some(candidates) = self.reader.missing_entry_point(self.record.entered()) {
            return Err(TaskFailure::stage_entry_missing(&candidates));
        }
        let rows = self.record.into_rows(&self.reader.stage_entries())?;
        Ok(Measurement::new(rows, Self::count_at(&summary, column)))
    }

    fn column_of(header: &str) -> Option<usize> {
        header
            .split_whitespace()
            .position(|event| event == Self::INSTRUCTIONS_EVENT)
    }

    fn count_at(counts: &[u64], column: usize) -> u64 {
        counts.get(column).copied().unwrap_or(0)
    }

    fn counts(fields: &str) -> Vec<u64> {
        fields
            .split_whitespace()
            .map(|field| field.parse().unwrap_or(0))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ProfileFixture;

    impl ProfileFixture {
        const ONE_CRATE: &'static str = include_str!("../../fixtures/bench/callgrind.out");
        const THREAD_ENTRY_COST: u64 = 1000;
        const NOTHING_COLLECTED: &'static str = "callgrind collected nothing under \
             `--toggle-collect=std::sys::backtrace::__rust_begin_short_backtrace`";
        const EMIT_ENTERED_UNDER_BUILD: &'static str = "\n\
             fn=compiler::build::formatting::EmittedFormatting::apply\n\
             cfn=compiler::emit::RustEmitter::emit\n\
             calls=1 0\n\
             0 7\n";
        const RENAMED_THREAD_ENTRY: &'static str = "events: Ir\n\
             summary: 900\n\
             fn=compiler::cli::Terminal::emit\n\
             cfn=compiler::expand::Expander::expand\n\
             calls=1 0\n\
             0 300\n\
             cfn=compiler::check::ProgramChecker::check\n\
             calls=1 0\n\
             0 200\n\
             cfn=compiler::emit::RustEmitter::emit\n\
             calls=1 0\n\
             0 100\n\
             cfn=compiler::build::formatting::EmittedFormatting::apply\n\
             calls=1 0\n\
             0 100\n\
             \n\
             fn=compiler::expand::Expander::expand\n\
             cfn=__rustc::__rust_alloc\n\
             calls=3 0\n\
             0 300\n";
        const TWO_CRATES: &'static str =
            include_str!("../../fixtures/bench/callgrind-two-crates.out");
        const COMPILER: [&'static str; 1] = ["compiler"];
        const COMPILER_AND_FRONT_END: [&'static str; 2] = ["compiler", "compiler_front"];
        const STAGE_ROWS: [(&'static str, u64); 6] = [
            ("build", 20),
            ("check", 300),
            ("emit", 50),
            ("expand", 500),
            ("other", 130),
            ("total", 1000),
        ];
        const BUILD_CALLS: &'static str = "\nfn=compiler::build::formatting::EmittedFormatting::apply\n\
             0 4\n\
             cfn=__rustc::__rust_alloc_zeroed\n\
             calls=1 0\n\
             0 5\n\
             cfn=alloc::raw_vec::RawVecInner<A>::reserve::do_reserve_and_handle\n\
             calls=1 0\n\
             0 3\n\
             cfn=compiler::build::token_alignment::RustTokens::scan\n\
             calls=1 0\n\
             0 8\n\
             \n\
             fn=compiler::build::token_alignment::RustTokens::scan\n\
             0 3\n\
             cfn=compiler::build::token_alignment::RustTokens::push\n\
             calls=1 0\n\
             0 5\n\
             \n\
             fn=compiler::build::token_alignment::RustTokens::push\n\
             0 3\n\
             cfn=__rustc::__rust_alloc\n\
             calls=1 0\n\
             0 2\n";

        const SHARED_CALLS: &'static str = "\nfn=compiler::emit::RustEmitter::emit\n\
             0 4\n\
             cfn=compiler::source::Text::copied\n\
             calls=1 0\n\
             0 6\n\
             \n\
             fn=compiler::build::formatting::EmittedFormatting::apply\n\
             0 4\n\
             cfn=compiler::source::Text::copied\n\
             calls=1 0\n\
             0 6\n\
             \n\
             fn=compiler::source::Text::copied\n\
             0 8\n\
             cfn=__rustc::__rust_alloc\n\
             calls=2 0\n\
             0 4\n";

        const ALLOCATOR_CALLING_THE_ALLOCATOR: &'static str = "\nfn=compiler::build::formatting::EmittedFormatting::apply\n\
             0 4\n\
             cfn=alloc::raw_vec::RawVec<T,A>::grow_one\n\
             calls=1 0\n\
             0 9\n\
             \n\
             fn=alloc::raw_vec::RawVec<T,A>::grow_one\n\
             0 2\n\
             cfn=__rustc::__rust_realloc\n\
             calls=1 0\n\
             0 7\n";

        const CALL_OUTSIDE_EVERY_STAGE: &'static str = "\nfn=compiler::build::formatting::EmittedFormatting::apply\n\
             0 4\n\
             cfn=core::ptr::drop_in_place<compiler::expand::Program>\n\
             calls=1 0\n\
             0 5\n\
             \n\
             fn=core::ptr::drop_in_place<compiler::expand::Program>\n\
             0 60\n\
             cfn=__rustc::__rust_dealloc\n\
             calls=1 0\n\
             0 2\n";

        const EMIT_CALL_UNDER_BUILD: &'static str = "\nfn=compiler::build::formatting::EmittedFormatting::apply\n\
             0 4\n\
             cfn=compiler::emit::writer::SpanMap::moved\n\
             calls=1 0\n\
             0 7\n\
             \n\
             fn=compiler::emit::writer::SpanMap::moved\n\
             0 4\n\
             cfn=__rustc::__rust_alloc\n\
             calls=1 0\n\
             0 3\n";

        fn read_with_thread_entry(
            profile: &str,
            crate_names: &[&str],
            entry_point_check: EntryPointCheck,
        ) -> Result<Measurement, TaskFailure> {
            let profile = format!(
                "{profile}{}",
                Self::thread_entry_call(Self::THREAD_ENTRY_COST)
            );
            Self::read(&profile, crate_names, entry_point_check)
        }

        fn read(
            profile: &str,
            crate_names: &[&str],
            entry_point_check: EntryPointCheck,
        ) -> Result<Measurement, TaskFailure> {
            let crate_names = crate_names.iter().map(ToString::to_string).collect();
            ProfileReader::new(&crate_names, entry_point_check).read(profile)
        }

        fn thread_entry_failure(profile: &str, entry_point_check: EntryPointCheck) -> String {
            Self::failure_text(Self::read(profile, &Self::COMPILER, entry_point_check))
        }

        fn failure_text(result: Result<Measurement, TaskFailure>) -> String {
            result.map_or_else(
                |failure| failure.to_string(),
                |measurement| format!("measured {:?}", Self::rows(&measurement)),
            )
        }

        fn thread_entry_call(cost: u64) -> String {
            format!(
                "\nfn=std::rt::lang_start::{{{{closure}}}}\n\
                 cfn=std::sys::backtrace::__rust_begin_short_backtrace\n\
                 calls=1 0\n\
                 0 {cost}\n"
            )
        }

        fn rows(measurement: &Measurement) -> Vec<(String, u64)> {
            measurement
                .rows()
                .map(|(stage, instructions)| (stage.to_string(), instructions))
                .collect()
        }

        fn stage_rows() -> Vec<(String, u64)> {
            Self::STAGE_ROWS
                .iter()
                .map(|(stage, instructions)| (stage.to_string(), *instructions))
                .collect()
        }

        fn with_calls(calls: &str) -> Measurement {
            let profile = format!("{}{calls}", Self::ONE_CRATE);
            Self::read_with_thread_entry(&profile, &Self::COMPILER, EntryPointCheck::Enforced)
                .unwrap()
        }

        fn without_emit_entry(profile: &str) -> String {
            profile.replace("RustEmitter::emit\n", "RustEmitter::lower\n")
        }
    }

    #[test]
    fn charges_calls_out_of_the_entry_module_to_the_callee_module() {
        let measurement = ProfileFixture::read_with_thread_entry(
            ProfileFixture::ONE_CRATE,
            &ProfileFixture::COMPILER,
            EntryPointCheck::Enforced,
        )
        .unwrap();
        assert_eq!(
            ProfileFixture::rows(&measurement),
            ProfileFixture::stage_rows()
        );
    }

    #[test]
    fn charges_calls_into_another_workspace_crate_to_the_callee_module() {
        let measurement = ProfileFixture::read_with_thread_entry(
            ProfileFixture::TWO_CRATES,
            &ProfileFixture::COMPILER_AND_FRONT_END,
            EntryPointCheck::Enforced,
        )
        .unwrap();
        assert_eq!(
            ProfileFixture::rows(&measurement),
            ProfileFixture::stage_rows()
        );
    }

    #[test]
    fn moves_an_allocator_call_below_a_stage_s_entry_point_to_its_allocator_row() {
        let measurement = ProfileFixture::with_calls(ProfileFixture::BUILD_CALLS);
        assert_eq!(measurement.instructions_of("build allocator"), Some(10));
        assert_eq!(measurement.instructions_of("build"), Some(10));
    }

    #[test]
    fn charges_allocation_to_the_stage_that_runs_it_not_the_module_that_defines_it() {
        let measurement = ProfileFixture::with_calls(ProfileFixture::EMIT_CALL_UNDER_BUILD);
        assert_eq!(measurement.instructions_of("emit allocator"), None);
        assert_eq!(measurement.instructions_of("emit"), Some(50));
        assert_eq!(measurement.instructions_of("build allocator"), Some(3));
        assert_eq!(measurement.instructions_of("build"), Some(17));
    }

    #[test]
    fn leaves_the_allocator_calls_of_a_function_two_stages_reach_in_both_stage_rows() {
        let measurement = ProfileFixture::with_calls(ProfileFixture::SHARED_CALLS);
        assert_eq!(measurement.instructions_of("emit allocator"), None);
        assert_eq!(measurement.instructions_of("build allocator"), None);
        assert_eq!(measurement.instructions_of("emit"), Some(50));
        assert_eq!(measurement.instructions_of("build"), Some(20));
    }

    #[test]
    fn counts_an_allocator_call_once_and_not_again_inside_the_allocator() {
        let measurement =
            ProfileFixture::with_calls(ProfileFixture::ALLOCATOR_CALLING_THE_ALLOCATOR);
        assert_eq!(measurement.instructions_of("build allocator"), Some(9));
        assert_eq!(measurement.instructions_of("build"), Some(11));
    }

    #[test]
    fn leaves_the_allocator_calls_of_a_function_the_code_outside_every_stage_reaches() {
        let measurement = ProfileFixture::with_calls(ProfileFixture::CALL_OUTSIDE_EVERY_STAGE);
        assert_eq!(measurement.instructions_of("build allocator"), None);
        assert_eq!(measurement.instructions_of("build"), Some(20));
    }

    #[test]
    fn rejects_a_profile_that_never_enters_a_stage() {
        let failure = ProfileFixture::read_with_thread_entry(
            &ProfileFixture::without_emit_entry(ProfileFixture::ONE_CRATE),
            &ProfileFixture::COMPILER,
            EntryPointCheck::Enforced,
        )
        .unwrap_err();
        assert!(
            failure
                .to_string()
                .contains("into `compiler::emit::RustEmitter::emit`,")
        );
    }

    #[test]
    fn names_the_missing_entry_point_in_each_workspace_crate() {
        let failure = ProfileFixture::read_with_thread_entry(
            &ProfileFixture::without_emit_entry(ProfileFixture::TWO_CRATES),
            &ProfileFixture::COMPILER_AND_FRONT_END,
            EntryPointCheck::Enforced,
        )
        .unwrap_err();
        assert!(failure.to_string().contains(
            "into `compiler::emit::RustEmitter::emit` or `compiler_front::emit::RustEmitter::emit`,"
        ));
    }

    #[test]
    fn skips_the_entry_points_of_a_baseline() {
        let measurement = ProfileFixture::read_with_thread_entry(
            &ProfileFixture::without_emit_entry(ProfileFixture::ONE_CRATE),
            &ProfileFixture::COMPILER,
            EntryPointCheck::Skipped,
        );
        assert!(measurement.is_ok());
    }

    #[test]
    fn sums_the_cost_of_every_call_into_the_thread_entry() {
        let profile = format!(
            "{}{}{}",
            ProfileFixture::ONE_CRATE,
            ProfileFixture::thread_entry_call(ProfileFixture::THREAD_ENTRY_COST),
            ProfileFixture::thread_entry_call(0)
        );
        let rows = ProfileFixture::read(
            &profile,
            &ProfileFixture::COMPILER,
            EntryPointCheck::Enforced,
        )
        .map(|measurement| ProfileFixture::rows(&measurement))
        .map_err(|failure| failure.to_string());
        assert_eq!(rows, Ok(ProfileFixture::stage_rows()));
    }

    #[test]
    fn names_the_thread_entry_toggle_when_no_call_enters_it() {
        let failure = ProfileFixture::thread_entry_failure(
            ProfileFixture::RENAMED_THREAD_ENTRY,
            EntryPointCheck::Skipped,
        );
        assert!(
            failure.contains(ProfileFixture::NOTHING_COLLECTED),
            "{failure}"
        );
    }

    #[test]
    fn names_the_thread_entry_toggle_when_its_calls_cost_nothing() {
        let profile = format!(
            "{}{}",
            ProfileFixture::RENAMED_THREAD_ENTRY,
            ProfileFixture::thread_entry_call(0)
        );
        let failure = ProfileFixture::thread_entry_failure(&profile, EntryPointCheck::Enforced);
        assert!(
            failure.contains(ProfileFixture::NOTHING_COLLECTED),
            "{failure}"
        );
    }

    #[test]
    fn names_the_thread_entry_toggle_before_a_missing_stage_entry() {
        let profile = ProfileFixture::without_emit_entry(ProfileFixture::RENAMED_THREAD_ENTRY);
        let failure = ProfileFixture::thread_entry_failure(&profile, EntryPointCheck::Enforced);
        assert!(
            failure.contains(ProfileFixture::NOTHING_COLLECTED),
            "{failure}"
        );
    }

    #[test]
    fn rejects_a_stage_entered_only_from_outside_the_command_line_module() {
        let profile = format!(
            "{}{}",
            ProfileFixture::without_emit_entry(ProfileFixture::ONE_CRATE),
            ProfileFixture::EMIT_ENTERED_UNDER_BUILD
        );
        let failure = ProfileFixture::failure_text(ProfileFixture::read_with_thread_entry(
            &profile,
            &ProfileFixture::COMPILER,
            EntryPointCheck::Enforced,
        ));
        assert!(
            failure.contains("into `compiler::emit::RustEmitter::emit`,"),
            "{failure}"
        );
    }

    #[test]
    fn rejects_a_profile_without_a_summary() {
        let failure = ProfileFixture::read_with_thread_entry(
            "events: Ir\n",
            &ProfileFixture::COMPILER,
            EntryPointCheck::Enforced,
        );
        assert!(failure.unwrap_err().to_string().contains("summary"));
    }
}
