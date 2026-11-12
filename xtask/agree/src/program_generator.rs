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
    fn defines_exactly_the_helpers_main_names_and_what_they_call() {
        for program in GeneratorFixture::generator().programs() {
            let text = program.entry_text();
            for template in HelperTemplate::ALL {
                let defined = text.contains(template.definition);
                let rest = text.replacen(template.definition, "", 1);
                let named = rest.contains(&format!("({} ", template.name))
                    || rest.contains(&format!(" {} ", template.name));
                assert!(
                    defined || !named,
                    "{} calls an undefined `{}`",
                    program.name,
                    template.name
                );
                assert!(
                    !defined || named,
                    "{} defines an unused `{}`",
                    program.name,
                    template.name
                );
            }
        }
    }
}
