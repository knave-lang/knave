use std::path::PathBuf;

use super::build_route::BuildRoute;
use super::cargo_route::CargoRoute;
use super::rustc_route::RustcRoute;
use crate::build::{CargoCompiler, CargoTarget};

pub struct BuildRoutes {
    shared_target: Box<dyn Fn() -> Option<PathBuf>>,
}

impl Default for BuildRoutes {
    fn default() -> Self {
        Self {
            shared_target: Box::new(CargoCompiler::shared_target),
        }
    }
}

impl BuildRoutes {
    #[cfg(test)]
    pub fn sharing(shared_target: PathBuf) -> Self {
        Self {
            shared_target: Box::new(move || Some(shared_target.clone())),
        }
    }

    pub fn preferring_cargo<'a>(
        &self,
        cargo: Option<CargoTarget<'a>>,
        rustc: RustcRoute<'a>,
    ) -> Box<dyn BuildRoute + 'a> {
        match cargo {
            Some(target) => Box::new(CargoRoute::new(target, (self.shared_target)())),
            None => Box::new(rustc),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::BuildRoutes;
    use crate::build::{BuildError, BuildProfile, CargoTarget};
    use crate::cli::crate_name::CrateName;
    use crate::cli::fake_crates::FakeCrates;
    use crate::cli::fake_programs::FakePrograms;
    use crate::cli::fake_stages::FakeStages;
    use crate::cli::project_crates::ProjectCrates;
    use crate::cli::recorded_output::RecordedOutput;
    use crate::cli::rustc_route::RustcRoute;
    use crate::cli::source_report::SourceReport;
    use crate::cli::temporary_directory::TemporaryDirectory;
    use crate::source::SourceMap;

    struct BuildRoutesFixture {
        root: TemporaryDirectory,
        crates: ProjectCrates,
        crate_name: CrateName,
        sources: SourceMap,
        output: RecordedOutput,
    }

    impl BuildRoutesFixture {
        const CARGO_OUTPUT: &str = "built-by-cargo";
        const RUSTC_OUTPUT: &str = "built-by-rustc";
        const RUSTC_DIRECTORY: &str = "rustc";
        const SHARED_TARGET: &str = "shared-target";

        fn new() -> Self {
            let root = TemporaryDirectory::create().expect("a temporary directory");
            let crates = FakeCrates::with_a_path_crate(root.path());
            Self {
                root,
                crates,
                crate_name: CrateName::of(Path::new(FakeCrates::PROJECT)),
                sources: SourceMap::default(),
                output: RecordedOutput::default(),
            }
        }

        fn cargo_output(&self) -> PathBuf {
            self.root.path().join(Self::CARGO_OUTPUT)
        }

        fn rustc_output(&self) -> PathBuf {
            self.root.path().join(Self::RUSTC_OUTPUT)
        }

        fn compile_preferring_cargo(
            &self,
            cargo: Option<CargoTarget<'_>>,
        ) -> Result<(), BuildError> {
            let rustc_output = self.rustc_output();
            let rustc = RustcRoute::new(
                self.root.path().join(Self::RUSTC_DIRECTORY),
                &self.crate_name,
                &rustc_output,
            );
            BuildRoutes::sharing(self.root.path().join(Self::SHARED_TARGET))
                .preferring_cargo(cargo, rustc)
                .compile(
                    &FakeStages::emitted(FakePrograms::PRINTS_A_SUM),
                    BuildProfile::Debug,
                    &SourceReport::new(&self.sources, &self.output),
                )
        }

        fn cargo_target(&self) -> Option<CargoTarget<'_>> {
            self.crates.program_target(
                FakeCrates::PROJECT,
                self.crates.build_directory(),
                self.cargo_output(),
            )
        }
    }

    #[test]
    fn a_cargo_target_takes_the_cargo_route() {
        let fixture = BuildRoutesFixture::new();
        assert!(
            fixture
                .compile_preferring_cargo(fixture.cargo_target())
                .is_ok()
        );
        assert!(fixture.cargo_output().is_file());
        assert!(!fixture.rustc_output().exists());
    }

    #[test]
    fn no_cargo_target_takes_the_rustc_route() {
        let fixture = BuildRoutesFixture::new();
        assert!(fixture.compile_preferring_cargo(None).is_ok());
        assert!(fixture.rustc_output().is_file());
        assert!(!fixture.cargo_output().exists());
    }
}
