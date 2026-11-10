use clap::Args;

use super::fmt::FmtMode;

#[derive(Args)]
pub struct CheckFlag {
    #[arg(
        long,
        help = "Write nothing, print each file that would change, and exit 1 if any would"
    )]
    check: bool,
}

impl CheckFlag {
    pub fn mode(&self) -> FmtMode {
        if self.check {
            FmtMode::Check
        } else {
            FmtMode::Write
        }
    }
}
