use std::path::{Path, PathBuf};

use super::build_route::BuildRoute;
use super::crate_name::CrateName;
use super::entry_span::EntrySpan;
use super::source_report::SourceReport;
use crate::build::{BuildError, BuildProfile, RustCompiler};
use crate::emit::EmittedCrate;

pub struct RustcRoute<'a> {
    directory: PathBuf,
    crate_name: &'a CrateName,
    output: &'a Path,
}

impl<'a> RustcRoute<'a> {
    pub fn new(directory: PathBuf, crate_name: &'a CrateName, output: &'a Path) -> Self {
        Self {
            directory,
            crate_name,
            output,
        }
    }
}

impl BuildRoute for RustcRoute<'_> {
    fn compile(
        &self,
        emitted: &EmittedCrate,
        profile: BuildProfile,
        _report: &SourceReport<'_>,
    ) -> Result<(), BuildError> {
        RustCompiler::new(self.directory.clone(), profile).compile(
            emitted,
            self.crate_name.as_str(),
            self.output,
            EntrySpan::start(),
        )
    }
}
