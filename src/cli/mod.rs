mod add;
mod build;
mod build_directory;
mod capped_report;
mod check;
mod check_flag;
mod checking;
mod checkout_bounds;
mod color_when;
mod command;
mod counter_report;
mod crate_section;
mod definition_buffer;
mod definition_rounds;
mod diagnostic_output;
mod distinct_output;
mod doc;
mod doc_arguments;
mod doc_folder;
mod doc_pages;
mod doc_write;
mod edited_lines;
mod emit;
mod enter_key;
mod entry_argument;
mod entry_tree;
mod error_location;
mod eval;
mod evaluation_output;
mod evaluation_request;
mod exit_status;
mod expand;
mod expansion;
#[cfg(test)]
mod fake_programs;
mod finished_program;
mod fmt;
mod front_end_stack;
mod input_helper;
mod inspection_command;
mod interruption;
mod json_replies;
mod library_walk;
mod located_program;
mod located_project;
mod module_of_sources;
mod name_records;
mod new;
mod open_input;
mod painted_program;
mod program_command;
mod program_file;
mod program_signal;
mod program_stream;
mod project_command;
mod project_crates;
mod project_file_edit;
mod project_task;
mod prompt_diagnostics;
mod prompt_history;
mod prompt_lines;
mod prompt_module;
#[cfg(test)]
mod recorded_output;
mod release_flag;
mod remove;
mod repl;
mod run;
mod skipped_directory;
mod source_report;
mod source_walk;
mod sources_survey;
mod stage_work;
mod standard_error;
mod standard_output;
mod streamed_program;
mod task;
mod task_catalog;
mod task_runners;
mod temporary_directory;
mod term_parser;
mod terminal;
mod test;
mod test_binary;
mod test_entries;
mod test_record;
mod test_report;
mod test_run;
mod token_colors;
mod tree_bounds;
mod types;
mod unreadable_file;
mod user_label_output;
mod value_marker;
mod whole_characters;

use std::env;
use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

use anstream::AutoStream;
use clap::{CommandFactory, FromArgMatches, Parser};

use crate::counters::CountingAllocator;
use crate::language::Language;
use color_when::ColorWhen;
use command::Command;
use counter_report::CounterReport;
use doc::DocCommand;
use front_end_stack::FrontEndStack;
use terminal::Terminal;

#[derive(Parser)]
#[command(
    name = Language::NAME,
    version,
    about = "Compile Lisp source files to Rust",
    after_help = "With no command, start the prompt as repl does."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long, global = true, hide = true)]
    counters: bool,
    #[arg(
        long,
        global = true,
        value_enum,
        value_name = "WHEN",
        default_value_t,
        help = "When to color the output"
    )]
    color: ColorWhen,
}

impl Cli {
    pub fn parse_arguments() -> Self {
        let arguments: Vec<OsString> = env::args_os().collect();
        let mut command = Self::command()
            .color(ColorWhen::written_in(&arguments).into())
            .mut_subcommand(DocCommand::NAME, |doc| {
                doc.override_usage(DocCommand::usage())
            });
        Self::parse_with(&mut command, arguments).unwrap_or_else(|error| error.exit())
    }

    fn parse_with(
        command: &mut clap::Command,
        arguments: Vec<OsString>,
    ) -> Result<Self, clap::Error> {
        let mut matches = command.try_get_matches_from_mut(arguments)?;
        Self::from_arg_matches_mut(&mut matches).map_err(|error| error.format(command))
    }

    pub fn run(self, allocator: Option<&CountingAllocator>) -> ExitCode {
        FrontEndStack::run(move || self.run_on_this_stack(allocator))
    }

    fn run_on_this_stack(self, allocator: Option<&CountingAllocator>) -> ExitCode {
        anstream::ColorChoice::from(self.color).write_global();
        let mut terminal = Terminal::new(AutoStream::choice(&io::stderr()));
        let command = self.command.unwrap_or_else(Command::prompt);
        let status = command.execute(&mut terminal);
        if self.counters {
            let mut report = CounterReport::new(allocator);
            report.add_stage_work(terminal.work());
            report.print();
        }
        status.die_by_the_program_signal();
        status.into()
    }
}
