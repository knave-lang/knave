use std::path::PathBuf;

use super::build_route::BuildRoute;
use super::entry_span::EntrySpan;
use super::source_report::SourceReport;
use crate::build::{BuildError, BuildPlan, BuildProfile, CargoCompiler, CargoTarget};
use crate::emit::EmittedCrate;

pub struct CargoRoute<'a> {
    target: CargoTarget<'a>,
    shared_target: Option<PathBuf>,
}

impl<'a> CargoRoute<'a> {
    pub fn new(target: CargoTarget<'a>, shared_target: Option<PathBuf>) -> Self {
        Self {
            target,
            shared_target,
        }
    }
}

impl BuildRoute for CargoRoute<'_> {
    fn compile(
        &self,
        emitted: &EmittedCrate,
        profile: BuildProfile,
        report: &SourceReport<'_>,
    ) -> Result<(), BuildError> {
        let waiting = |directory: &str| {
            report.note(&format!("waiting for another build in {directory}"));
        };
        CargoCompiler::new(
            self.shared_target.clone(),
            BuildPlan::new(profile),
            &waiting,
        )
        .compile(emitted, &self.target, EntrySpan::start())
    }
}
