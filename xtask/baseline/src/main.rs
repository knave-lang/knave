use std::env;
use std::process::ExitCode;

use knave_baseline::MergeCommand;

fn main() -> ExitCode {
    MergeCommand::run(env::args().skip(1))
}
