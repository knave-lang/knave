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
