use std::path::PathBuf;

use crate::main_body::MainBody;
use crate::program_source::ProgramSource;
use crate::seed::{Seed, SeedRange};
use crate::subject_program::SubjectProgram;

pub struct ProgramGenerator {
    seeds: SeedRange,
    main: MainBody,
}

impl ProgramGenerator {
    const ENTRY: &str = "main.knv";

    pub fn new(seeds: SeedRange, statements: usize) -> Self {
        Self {
            seeds,
            main: MainBody::of_statements(statements),
        }
    }

    pub fn program(&self, seed: Seed) -> SubjectProgram {
        let text = self.main.write(seed).program_text();
        SubjectProgram::generated(seed, PathBuf::from(Self::ENTRY), text)
    }
}

impl ProgramSource for ProgramGenerator {
    fn programs(&self) -> Box<dyn Iterator<Item = SubjectProgram> + '_> {
        Box::new(self.seeds.seeds().map(|seed| self.program(seed)))
    }
}

#[cfg(test)]
mod tests {
    use super::ProgramGenerator;
    use crate::helper_template::HelperTemplate;
    use crate::program_source::ProgramSource;
    use crate::seed::{Seed, SeedRange};

    struct GeneratorFixture;

    impl GeneratorFixture {
        const STATEMENTS: usize = 14;
        const SEEDS: u64 = 200;

        fn generator() -> ProgramGenerator {
            ProgramGenerator::new(SeedRange::new(0..Self::SEEDS), Self::STATEMENTS)
        }

        fn is_helper(name: &str) -> bool {
            HelperTemplate::ALL
                .iter()
                .any(|template| template.name == name)
        }
    }

    #[test]
    fn one_seed_gives_one_program_named_after_it() {
        let generator = GeneratorFixture::generator();
        let program = generator.program(Seed::from(3));
        assert_eq!(program, generator.program(Seed::from(3)));
        assert_ne!(program.files, generator.program(Seed::from(4)).files);
        assert_eq!(program.name, "seed-3");
        assert_eq!(program.seed, Some(Seed::from(3)));
    }

    #[test]
    fn every_program_prints_and_never_reaches_a_hash_map() {
        for program in GeneratorFixture::generator().programs() {
            assert!(program.forms.prelude_names.contains_key("println"));
            assert!(
                !program
                    .forms
                    .prelude_names
                    .keys()
                    .any(|name| name.contains("hash-map"))
            );
        }
    }

    #[test]
    fn defines_only_the_helpers_main_calls() {
        for program in GeneratorFixture::generator().programs() {
            let census = &program.forms.prelude_names;
            let defined = census.get("defn").copied().unwrap_or_default();
            let called = census
                .keys()
                .filter(|name| GeneratorFixture::is_helper(name))
                .count();
            assert_eq!(defined, called + 1, "{}", program.name);
        }
    }
}
