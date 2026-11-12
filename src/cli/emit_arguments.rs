use clap::Args;

use super::emit::EmitCommand;
use super::entry_argument::EntryArgument;
use super::exit_status::ExitStatus;
use super::terminal::Terminal;
use crate::ownership::{EnforcedOwnership, WaivedOwnership};

#[derive(Args)]
pub struct EmitArguments {
    #[command(flatten)]
    entry: EntryArgument,
    #[arg(long, hide = true)]
    waive_ownership_refusals: bool,
}

impl EmitArguments {
    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        if self.waive_ownership_refusals {
            EmitCommand::<WaivedOwnership>::new(self.entry).run(terminal)
        } else {
            EmitCommand::<EnforcedOwnership>::new(self.entry).run(terminal)
        }
    }
}
