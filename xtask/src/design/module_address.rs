use std::collections::BTreeMap;

use crate::parsed_source::ParsedSource;

pub struct Stage {
    crate_directory: &'static str,
    module: &'static str,
}

impl Stage {
    pub const fn new(crate_directory: &'static str, module: &'static str) -> Self {
        Self {
            crate_directory,
            module,
        }
    }

    pub fn contains(&self, address: &ModuleAddress) -> bool {
        address.crate_directory == self.crate_directory
            && address
                .segments
                .first()
                .is_some_and(|top| top == self.module)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleAddress {
    crate_directory: String,
    segments: Vec<String>,
}

impl ModuleAddress {
    const CRATE: &str = "crate";
    const SELF: &str = "self";
    const SUPER: &str = "super";
    const CRATE_ROOTS: [&str; 2] = ["lib", "main"];
    const DIRECTORY_MODULE: &str = "mod";
    const EXTENSION: &str = ".rs";

    pub fn of(source: &ParsedSource) -> Self {
        let within = source
            .file()
            .strip_prefix(source.crate_directory())
            .unwrap_or(source.file())
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
        Self {
            crate_directory: source.crate_directory().to_string(),
            segments,
        }
    }

    fn within(&self, segments: Vec<String>) -> Self {
        Self {
            crate_directory: self.crate_directory.clone(),
            segments,
        }
    }

    fn resolution(&self, path: &[String]) -> Option<Resolution> {
        let (first, rest) = path.split_first()?;
        match first.as_str() {
            Self::CRATE => Some(Resolution::anywhere(rest.to_vec())),
            Self::SELF => Some(self.below(rest)),
            Self::SUPER => self.above(path),
            _ => Some(self.below(path)),
        }
    }

    fn below(&self, path: &[String]) -> Resolution {
        Resolution {
            absolute: [&self.segments[..], path].concat(),
            shortest: self.segments.len() + 1,
        }
    }

    fn above(&self, path: &[String]) -> Option<Resolution> {
        let levels = path
            .iter()
            .take_while(|segment| *segment == Self::SUPER)
            .count();
        let base = self.segments.len().checked_sub(levels)?;
        Some(Resolution::anywhere(
            [&self.segments[..base], &path[levels..]].concat(),
        ))
    }
}

struct Resolution {
    absolute: Vec<String>,
    shortest: usize,
}

impl Resolution {
    const SHORTEST_MODULE_PATH: usize = 1;

    fn anywhere(absolute: Vec<String>) -> Self {
        Self {
            absolute,
            shortest: Self::SHORTEST_MODULE_PATH,
        }
    }
}

pub struct ModuleFiles {
    addresses: BTreeMap<String, ModuleAddress>,
    files: BTreeMap<ModuleAddress, String>,
}

impl ModuleFiles {
    pub fn build(sources: &[ParsedSource]) -> Self {
        let addresses: BTreeMap<String, ModuleAddress> = sources
            .iter()
            .map(|source| (source.file().to_string(), ModuleAddress::of(source)))
            .collect();
        let files = addresses
            .iter()
            .map(|(file, address)| (address.clone(), file.clone()))
            .collect();
        Self { addresses, files }
    }

    pub fn files(&self) -> impl Iterator<Item = &String> {
        self.addresses.keys()
    }

    pub fn address(&self, file: &str) -> &ModuleAddress {
        &self.addresses[file]
    }

    pub fn resolve(&self, file: &str, path: &[String]) -> Option<&String> {
        self.locate(file, path).map(|(found, _)| found)
    }

    pub fn locate(&self, file: &str, path: &[String]) -> Option<(&String, Vec<String>)> {
        let address = self.address(file);
        let resolution = address.resolution(path)?;
        let absolute = &resolution.absolute;
        (resolution.shortest..=absolute.len())
            .rev()
            .find_map(|length| {
                self.files
                    .get(&address.within(absolute[..length].to_vec()))
                    .map(|found| (found, absolute[length..].to_vec()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleAddress;
    use crate::parsed_source::ParsedSourceFixture;

    struct Modules;

    impl Modules {
        fn address_of(file: &str) -> Vec<String> {
            ModuleAddress::of(&ParsedSourceFixture::sample(file, "")).segments
        }
    }

    #[test]
    fn a_crate_root_is_the_empty_module() {
        assert!(Modules::address_of("src/lib.rs").is_empty());
        assert!(Modules::address_of("expand/src/main.rs").is_empty());
    }

    #[test]
    fn a_file_names_its_module_by_its_path() {
        assert_eq!(
            Modules::address_of("expand/src/core/kernel.rs"),
            ["core", "kernel"]
        );
    }

    #[test]
    fn a_directory_module_drops_its_file_name() {
        assert_eq!(Modules::address_of("src/cli/mod.rs"), ["cli"]);
    }

    #[test]
    fn a_module_named_like_a_crate_root_below_the_root_keeps_its_name() {
        assert_eq!(Modules::address_of("src/build/main.rs"), ["build", "main"]);
    }
}
