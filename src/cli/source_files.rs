use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use super::diagnostic_output::StandardErrorMessages;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::program_file::ProgramFile;
use super::skipped_directory::SkippedDirectory;
use super::tree_bounds::TreeBounds;
use super::unreadable_file::UnreadableFile;
use super::working_directory::WorkingDirectory;
use crate::language::Language;
use crate::modules::ProjectTree;

pub struct SourceFiles {
    root: PathBuf,
    bounds: TreeBounds,
    skipped: SkippedDirectory,
    left_out: Vec<PathBuf>,
}

#[derive(Default)]
struct Visit {
    real_folders: FxHashSet<PathBuf>,
    walked_folders: FxHashSet<PathBuf>,
    seen: FxHashSet<PathBuf>,
    found: Vec<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    RealPaths,
    Links,
}

impl SourceFiles {
    pub fn under(root: PathBuf, project: &LocatedProject) -> Self {
        let root = Self::walked_root(root);
        let placement = project.placement();
        let left_out = [placement.tasks_directory(), placement.tests_directory()]
            .iter()
            .filter_map(|directory| ProgramFile::canonical(directory))
            .collect();
        Self {
            skipped: SkippedDirectory::for_walk_of(&root),
            bounds: TreeBounds::of(&root),
            root,
            left_out,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn paths(&self) -> Result<Vec<PathBuf>, ExitStatus> {
        let mut visit = Visit::default();
        visit
            .real_folders
            .extend(ProgramFile::canonical(&self.root));
        self.collect(&self.root, &mut visit, Pass::RealPaths)?;
        visit.found.sort();
        let real_paths = visit.found.len();
        self.collect(&self.root, &mut visit, Pass::Links)?;
        visit.found[real_paths..].sort();
        Ok(visit.found)
    }

    fn walked_root(root: PathBuf) -> PathBuf {
        if root.as_os_str().is_empty() {
            WorkingDirectory::path().to_path_buf()
        } else {
            root
        }
    }

    fn collect(&self, directory: &Path, visit: &mut Visit, pass: Pass) -> Result<(), ExitStatus> {
        let unreadable = |error: io::Error| {
            UnreadableFile::new(directory, &error).report_to(&StandardErrorMessages)
        };
        for entry in fs::read_dir(directory).map_err(unreadable)? {
            self.take(entry.map_err(unreadable)?.path(), visit, pass)?;
        }
        Ok(())
    }

    fn take(&self, path: PathBuf, visit: &mut Visit, pass: Pass) -> Result<(), ExitStatus> {
        if pass == Pass::RealPaths && path.is_symlink() {
            return Ok(());
        }
        if path.is_dir() {
            return self.collect_inside(&path, visit, pass);
        }
        if Self::is_source(&path) && visit.seen.insert(path.clone()) {
            visit.found.push(path);
        }
        Ok(())
    }

    fn collect_inside(
        &self,
        directory: &Path,
        visit: &mut Visit,
        pass: Pass,
    ) -> Result<(), ExitStatus> {
        let Some(canonical) = self.walked_folder(directory) else {
            return Ok(());
        };
        let first_walk = match pass {
            Pass::RealPaths => visit.real_folders.insert(canonical),
            Pass::Links => {
                !(directory.is_symlink() && visit.real_folders.contains(&canonical))
                    && visit.walked_folders.insert(canonical)
            }
        };
        if first_walk {
            self.collect(directory, visit, pass)?;
        }
        Ok(())
    }

    fn walked_folder(&self, directory: &Path) -> Option<PathBuf> {
        self.walks_into(directory)
            .then(|| ProgramFile::canonical(directory))
            .flatten()
    }

    fn walks_into(&self, directory: &Path) -> bool {
        match self.bounds.inside(directory) {
            Some(directory) => {
                !self.skipped.is_skipped(&directory)
                    && !self.left_out.contains(&directory)
                    && !ProjectTree::holds_a_project(&directory)
            }
            None => false,
        }
    }

    fn is_source(file: &Path) -> bool {
        file.extension()
            .is_some_and(|extension| extension == Language::EXTENSION)
            && !ProgramFile::is_project_file(file)
    }
}
