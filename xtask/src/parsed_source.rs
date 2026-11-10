use crate::source_file::SourceFile;
use crate::test_code::TestCode;
use crate::test_modules::TestModuleFiles;

pub struct ParsedSource {
    crate_directory: String,
    file: String,
    syntax: syn::File,
}

impl ParsedSource {
    const CRATE_ROOTS: [&str; 2] = ["lib", "main"];
    const DIRECTORY_MODULE: &str = "mod";
    const EXTENSION: &str = ".rs";

    pub fn all_production(sources: Vec<SourceFile>, crate_directories: &[String]) -> Vec<Self> {
        let test_files = TestModuleFiles::find(&sources);
        sources
            .into_iter()
            .filter(|source| {
                !test_files.contains(source) && !TestCode::marks(&source.syntax().attrs)
            })
            .map(|source| {
                let crate_directory = Self::crate_directory_of(&source, crate_directories);
                let file = source.relative().to_string();
                Self::stripped(crate_directory, file, source.into_syntax())
            })
            .collect()
    }

    fn crate_directory_of(source: &SourceFile, crate_directories: &[String]) -> String {
        crate_directories
            .iter()
            .filter(|directory| source.relative().starts_with(&format!("{directory}/")))
            .max_by_key(|directory| directory.len())
            .cloned()
            .unwrap_or_default()
    }

    fn stripped(crate_directory: String, file: String, mut syntax: syn::File) -> Self {
        TestCode::strip(&mut syntax);
        Self {
            crate_directory,
            file,
            syntax,
        }
    }

    pub fn crate_directory(&self) -> &str {
        &self.crate_directory
    }

    pub fn file(&self) -> &str {
        &self.file
    }

    pub fn syntax(&self) -> &syn::File {
        &self.syntax
    }

    pub fn module(&self) -> Vec<String> {
        let within = self
            .file
            .strip_prefix(&self.crate_directory)
            .unwrap_or(&self.file)
            .trim_start_matches('/')
            .trim_end_matches(Self::EXTENSION);
        let mut segments: Vec<String> = within.split('/').map(String::from).collect();
        let is_crate_root = segments.len() == 1 && Self::CRATE_ROOTS.contains(&within);
        if is_crate_root
            || segments
                .last()
                .is_some_and(|last| last == Self::DIRECTORY_MODULE)
        {
            segments.pop();
        }
        segments
    }
}

#[cfg(test)]
impl ParsedSource {
    const SOURCE_DIRECTORY: &str = "src";

    pub fn sample(file: &str, text: &str) -> Self {
        let start = file
            .find(Self::SOURCE_DIRECTORY)
            .expect("the file lies in a src directory");
        let crate_directory = file[..start + Self::SOURCE_DIRECTORY.len()].to_string();
        let syntax = syn::parse_file(text).expect("the sample parses");
        Self::stripped(crate_directory, file.to_string(), syntax)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ParsedSource;
    use crate::source_file::SourceFile;

    struct Fixture;

    impl Fixture {
        fn source(path: &str, text: &str) -> SourceFile {
            SourceFile::parse(PathBuf::from(path), path.to_string(), text)
                .expect("the source parses")
        }
    }

    struct ModuleOf;

    impl ModuleOf {
        fn file(file: &str) -> Vec<String> {
            ParsedSource::sample(file, "").module()
        }
    }

    #[test]
    fn a_crate_root_is_the_empty_module() {
        assert!(ModuleOf::file("src/lib.rs").is_empty());
        assert!(ModuleOf::file("expand/src/main.rs").is_empty());
    }

    #[test]
    fn a_file_names_its_module_by_its_path() {
        assert_eq!(
            ModuleOf::file("expand/src/core/kernel.rs"),
            ["core", "kernel"]
        );
    }

    #[test]
    fn a_directory_module_drops_its_file_name() {
        assert_eq!(ModuleOf::file("src/cli/mod.rs"), ["cli"]);
    }

    #[test]
    fn a_module_named_like_a_crate_root_below_the_root_keeps_its_name() {
        assert_eq!(ModuleOf::file("src/build/main.rs"), ["build", "main"]);
    }

    #[test]
    fn loads_production_files_with_their_crate_directory() {
        let sources = vec![
            Fixture::source(
                "expand/src/lib.rs",
                "#[cfg(test)]\nmod fixture;\nfn kept() {}",
            ),
            Fixture::source("expand/src/fixture.rs", "fn helper() {}"),
            Fixture::source("src/only_tests.rs", "#![cfg(test)]\nfn helper() {}"),
            Fixture::source("src/a.rs", ""),
        ];
        let directories = ["src".to_string(), "expand/src".to_string()];
        let parsed = ParsedSource::all_production(sources, &directories);
        let files: Vec<(&str, &str)> = parsed
            .iter()
            .map(|source| (source.crate_directory.as_str(), source.file.as_str()))
            .collect();
        assert_eq!(
            files,
            [("expand/src", "expand/src/lib.rs"), ("src", "src/a.rs")]
        );
    }

    #[test]
    fn a_file_belongs_to_the_innermost_crate_directory() {
        let sources = vec![Fixture::source("src/build/src/a.rs", "")];
        let directories = ["src".to_string(), "src/build/src".to_string()];
        let parsed = ParsedSource::all_production(sources, &directories);
        assert_eq!(parsed[0].crate_directory, "src/build/src");
    }

    #[test]
    fn test_items_are_gone() {
        let source = ParsedSource::sample("src/a.rs", "#[cfg(test)] mod tests {} fn kept() {}");
        assert_eq!(source.syntax.items.len(), 1);
    }
}
