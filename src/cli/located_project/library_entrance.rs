use super::LocatedProject;
use crate::cli::exit_status::ExitStatus;
use crate::cli::library_walk::LibraryStore;
use crate::cli::terminal::Terminal;
use crate::cli::walk_start::WalkStart;

pub struct LibraryEntrance;

impl LibraryEntrance {
    pub fn enter(
        start: WalkStart<'_>,
        terminal: &Terminal,
        store: impl LibraryStore,
    ) -> Result<LocatedProject, ExitStatus> {
        LocatedProject::walked_from(start, terminal, store)
    }
}
