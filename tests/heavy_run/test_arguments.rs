use std::env;
use std::marker::PhantomData;

use super::narrowing::Narrowing;

pub trait FilterRule {
    fn narrows(filters: &[&str], exact: bool) -> bool;
}

pub struct TestArguments<Rule> {
    arguments: Vec<String>,
    rule: PhantomData<Rule>,
}

impl<Rule: FilterRule> TestArguments<Rule> {
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
    const OPTION_PREFIX: char = '-';

    pub fn from_process() -> Self {
        Self::of(env::args().skip(1))
    }

    pub fn of(arguments: impl IntoIterator<Item = String>) -> Self {
        Self {
            arguments: arguments.into_iter().collect(),
            rule: PhantomData,
        }
    }

    fn filters(&self) -> Vec<&str> {
        let mut filters = Vec::new();
        let mut arguments = self.arguments.iter();
        while let Some(argument) = arguments.next() {
            if Self::VALUED_OPTIONS.contains(&argument.as_str()) {
                arguments.next();
            } else if !argument.starts_with(Self::OPTION_PREFIX) {
                filters.push(argument.as_str());
            }
        }
        filters
    }

    fn exact(&self) -> bool {
        self.arguments
            .iter()
            .any(|argument| argument == Self::EXACT)
    }
}

impl<Rule: FilterRule> Narrowing for TestArguments<Rule> {
    fn narrows(&self) -> bool {
        Rule::narrows(&self.filters(), self.exact())
    }
}
