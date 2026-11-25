use std::io::{self, Write};

pub struct UncapturedOutput;

impl UncapturedOutput {
    pub fn write(line: &str) {
        let _ = writeln!(io::stderr(), "{line}");
    }
}
