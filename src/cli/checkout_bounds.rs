use std::path::{Component, Path, PathBuf};

use super::program_file::ProgramFile;

pub enum RepositoryAddress<'u> {
    Remote(&'u str),
    Local(PathBuf),
}

#[derive(Clone)]
pub struct CheckoutBounds {
    root: PathBuf,
}

impl<'u> RepositoryAddress<'u> {
    const SCHEME_SEPARATOR: &'static str = "://";
    const FILE_SCHEME: &'static str = "file";
    const HOST_SEPARATOR: char = ':';
    const PATH_SEPARATOR: char = '/';

    pub fn of(url: &'u str) -> Self {
        if let Some((scheme, rest)) = url.split_once(Self::SCHEME_SEPARATOR) {
            return match scheme {
                Self::FILE_SCHEME => Self::Local(PathBuf::from(rest)),
                _ => Self::Remote(url),
            };
        }
        if Self::is_host_and_path(url) {
            return Self::Remote(url);
        }
        Self::Local(PathBuf::from(url))
    }

    fn is_host_and_path(url: &str) -> bool {
        let Some(host_end) = url.find(Self::HOST_SEPARATOR) else {
            return false;
        };
        !url[..host_end].contains(Self::PATH_SEPARATOR)
    }
}

impl CheckoutBounds {
    pub fn of(checkout: &Path) -> Self {
        Self {
            root: Self::resolved(checkout),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn contains(&self, path: &Path) -> bool {
        Self::resolved(path).starts_with(&self.root)
    }

    fn resolved(path: &Path) -> PathBuf {
        path.ancestors()
            .find_map(|ancestor| {
                let canonical = ProgramFile::canonical(ancestor)?;
                let rest = path.strip_prefix(ancestor).ok()?;
                Some(Self::without_parent_steps(&canonical.join(rest)))
            })
            .unwrap_or_else(|| Self::without_parent_steps(path))
    }

    fn without_parent_steps(path: &Path) -> PathBuf {
        let mut resolved = PathBuf::new();
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    resolved.pop();
                }
                Component::CurDir => {}
                other => resolved.push(other),
            }
        }
        resolved
    }
}
