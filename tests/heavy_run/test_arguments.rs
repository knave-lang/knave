use std::env;

use super::selection::Selection;

pub struct TestArguments {
    arguments: Vec<String>,
}

impl TestArguments {
    const VALUED_OPTIONS: [&'static str; 7] = [
        "--skip",
        "--test-threads",
        "--color",
        "--format",
        "--logfile",
        "--shuffle-seed",
        "-Z",
    ];
    const EXACT: &'static str = "--exact";
    const END_OF_OPTIONS: &'static str = "--";
    const OPTION_PREFIX: char = '-';

    pub fn from_process() -> Self {
        Self::of(env::args().skip(1))
    }

    pub fn of(arguments: impl IntoIterator<Item = String>) -> Self {
        Self {
            arguments: arguments.into_iter().collect(),
        }
    }

    pub fn selection(&self) -> Selection {
        let mut filters = Vec::new();
        let mut exact = false;
        let mut arguments = self.arguments.iter();
        while let Some(argument) = arguments.next() {
            if argument == Self::END_OF_OPTIONS {
                filters.extend(arguments.by_ref().cloned());
            } else if argument == Self::EXACT {
                exact = true;
            } else if Self::VALUED_OPTIONS.contains(&argument.as_str()) {
                arguments.next();
            } else if !argument.starts_with(Self::OPTION_PREFIX) {
                filters.push(argument.clone());
            }
        }
        Selection { filters, exact }
    }
}
