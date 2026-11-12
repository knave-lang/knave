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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::RustcRoute;
    use crate::build::{BuildError, BuildProfile};
    use crate::cli::build_route::BuildRoute;
    use crate::cli::crate_name::CrateName;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::source_report::SourceReport;
    use crate::cli::temporary_directory::TemporaryDirectory;
    use crate::source::SourceMap;

    struct RustcFixture {
        directory: TemporaryDirectory,
        crate_name: CrateName,
        sources: SourceMap,
        output: RecordedOutput,
    }

    impl RustcFixture {
        const CRATE_STEM: &str = "geometry";

        fn new() -> Self {
            Self {
                directory: TemporaryDirectory::create().expect("a temporary directory"),
                crate_name: CrateName::of(Path::new(Self::CRATE_STEM)),
                sources: SourceMap::default(),
                output: RecordedOutput::default(),
            }
        }

        fn binary(&self) -> PathBuf {
            self.crate_name.path_in(self.directory.path())
        }

        fn compile(&self, text: &str) -> Result<(), BuildError> {
            let binary = self.binary();
            RustcRoute::new(
                self.directory.path().to_path_buf(),
                &self.crate_name,
                &binary,
            )
            .compile(
                &FakeStages::emitted(text),
                BuildProfile::Debug,
                &SourceReport::new(&self.sources, &self.output),
            )
        }
    }

    #[test]
    fn compiles_the_emitted_crate_into_its_output() {
        let fixture = RustcFixture::new();
        assert!(fixture.compile(FakePrograms::PRINTS_A_SUM).is_ok());
        assert!(fixture.binary().is_file());
    }
}
