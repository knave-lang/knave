use std::collections::BTreeMap;
use std::fmt;

use crate::backend_verdicts::BackendVerdicts;
use crate::form_census::FormCensus;

#[derive(Default)]
pub struct RunTally {
    programs: usize,
    outcomes: BTreeMap<TallyKey, usize>,
    codes: BTreeMap<TallyKey, usize>,
    forms: FormCensus,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TallyKey {
    backend: &'static str,
    label: String,
}

impl RunTally {
    pub fn count(&mut self, verdicts: &BackendVerdicts) {
        self.programs += 1;
        self.forms.add(&verdicts.program.forms);
        for (backend, verdict) in &verdicts.verdicts {
            Self::add(&mut self.outcomes, backend, verdict.outcome_name());
            for rejection in verdict.rejections() {
                Self::add(&mut self.codes, backend, rejection.code_text());
            }
        }
    }

    fn add(counts: &mut BTreeMap<TallyKey, usize>, backend: &'static str, label: &str) {
        let key = TallyKey {
            backend,
            label: label.to_string(),
        };
        *counts.entry(key).or_default() += 1;
    }
}

impl fmt::Display for TallyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.backend, self.label)
    }
}

impl fmt::Display for RunTally {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "{} programs", self.programs)?;
        for (key, count) in self.outcomes.iter().chain(&self.codes) {
            writeln!(formatter, "  {key}: {count}")?;
        }
        write!(formatter, "{}", self.forms)
    }
}
