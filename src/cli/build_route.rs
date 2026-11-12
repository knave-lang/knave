use super::source_report::SourceReport;
use crate::build::{BuildError, BuildProfile};
use crate::emit::EmittedCrate;

pub trait BuildRoute {
    fn compile(
        &self,
        emitted: &EmittedCrate,
        profile: BuildProfile,
        report: &SourceReport<'_>,
    ) -> Result<(), BuildError>;
}
