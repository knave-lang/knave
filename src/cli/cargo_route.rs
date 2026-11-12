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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CargoRoute;
    use crate::build::{BuildError, BuildProfile};
    use crate::cli::build_route::BuildRoute;
    use crate::cli::fake_crates::FakeCrates;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::project_crates::ProjectCrates;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::source_report::SourceReport;
    use crate::cli::temporary_directory::TemporaryDirectory;
    use crate::source::SourceMap;

    struct CargoRouteFixture {
        root: TemporaryDirectory,
        crates: ProjectCrates,
        sources: SourceMap,
        output: RecordedOutput,
    }

    impl CargoRouteFixture {
        const SHARED_TARGET: &str = "shared-target";

        fn new() -> Self {
            let root = TemporaryDirectory::create().expect("a temporary directory");
            let crates = FakeCrates::with_a_path_crate(root.path());
            Self {
                root,
                crates,
                sources: SourceMap::default(),
                output: RecordedOutput::default(),
            }
        }

        fn shared_target(&self) -> PathBuf {
            self.root.path().join(Self::SHARED_TARGET)
        }

        fn binary(&self) -> PathBuf {
            self.crates.build_directory().join(FakeCrates::PROJECT)
        }

        fn compile(&self, text: &str) -> Result<(), BuildError> {
            let target = self
                .crates
                .program_target(
                    FakeCrates::PROJECT,
                    self.crates.build_directory(),
                    self.binary(),
                )
                .expect("a project with a crate builds through cargo");
            CargoRoute::new(target, Some(self.shared_target())).compile(
                &FakeStages::emitted(text),
                BuildProfile::Debug,
                &SourceReport::new(&self.sources, &self.output),
            )
        }
    }

    #[test]
    fn compiles_the_emitted_crate_with_its_project_crates_in_the_shared_target() {
        let fixture = CargoRouteFixture::new();
        assert!(fixture.compile(FakePrograms::PRINTS_A_SUM).is_ok());
        assert!(fixture.binary().is_file());
        assert!(fixture.binary().starts_with(fixture.root.path()));
        assert!(fixture.shared_target().is_dir());
    }
}
