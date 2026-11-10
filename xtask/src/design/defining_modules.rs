use std::collections::{BTreeMap, BTreeSet};

use super::module_address::ModuleFiles;
use super::module_names::ModuleNames;
use crate::parsed_source::ParsedSource;

pub struct DefiningModules<'a> {
    modules: &'a ModuleFiles,
    names: BTreeMap<&'a str, ModuleNames>,
}

impl<'a> DefiningModules<'a> {
    pub fn build(sources: &'a [ParsedSource], modules: &'a ModuleFiles) -> Self {
        let names = sources
            .iter()
            .map(|source| (source.file(), ModuleNames::of(source.syntax())))
            .collect();
        Self { modules, names }
    }

    pub fn target(&self, file: &str, path: &[String]) -> Option<&'a str> {
        let path = self.through_binding(file, path);
        let (named, rest) = self.modules.locate(file, &path)?;
        let named = named.as_str();
        let defining = self.follow(named, &rest, &mut BTreeSet::new());
        Some(defining.unwrap_or(named))
    }

    fn through_binding(&self, file: &str, path: &[String]) -> Vec<String> {
        let bound = path.split_first().and_then(|(first, rest)| {
            let binding = self.names.get(file)?.bindings().path_of(first)?;
            Some([binding, rest].concat())
        });
        bound.unwrap_or_else(|| path.to_vec())
    }

    fn follow(
        &self,
        named: &'a str,
        rest: &[String],
        visited: &mut BTreeSet<&'a str>,
    ) -> Option<&'a str> {
        let Some(name) = rest.first() else {
            return Some(named);
        };
        let names = &self.names[named];
        if names.defines(name) {
            return Some(named);
        }
        if !visited.insert(named) {
            return None;
        }
        let next = match names.bindings().path_of(name) {
            Some(bound) => self
                .modules
                .locate(named, &[bound, &rest[1..]].concat())
                .map(|(module, inner)| (module.as_str(), inner)),
            None => self
                .glob_offering(named, name, visited)
                .map(|module| (module, rest.to_vec())),
        };
        let (module, inner) = next?;
        self.follow(module, &inner, visited)
    }

    fn glob_offering(
        &self,
        named: &'a str,
        name: &str,
        visited: &mut BTreeSet<&'a str>,
    ) -> Option<&'a str> {
        self.names[named]
            .bindings()
            .globs()
            .iter()
            .find_map(|glob| {
                let (module, inner) = self.modules.locate(named, glob)?;
                let module = module.as_str();
                if !inner.is_empty() {
                    return None;
                }
                if self.names[module].offers(name) {
                    return Some(module);
                }
                if !visited.insert(module) {
                    return None;
                }
                self.glob_offering(module, name, visited)
            })
    }
}
