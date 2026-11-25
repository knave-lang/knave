use std::io::{self, Write};

pub struct Uncaptured;

impl Uncaptured {
    pub fn write(line: &str) {
        let _ = writeln!(io::stderr(), "{line}");
    }
}
