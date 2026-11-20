use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::module_names::ModuleNames;
use super::source_files::SourceFiles;
use super::taken_module_paths::TakenModulePaths;
use super::terminal::Terminal;
use super::tree_file::TreeFile;

pub struct SourceWalk<'p> {
    files: SourceFiles,
    taken: TakenModulePaths<'p>,
}

impl<'p> SourceWalk<'p> {
    pub fn of(project: &'p LocatedProject) -> Self {
        Self {
            files: SourceFiles::under(project.source_root(), project),
            taken: project.placement().taken_module_paths(),
        }
    }

    pub fn module_paths(&self, terminal: &mut Terminal) -> Result<Vec<String>, ExitStatus> {
        let mut files: Vec<TreeFile> = self
            .files
            .paths()?
            .into_iter()
            .filter_map(|path| {
                let module = ModuleNames::of_source(path.strip_prefix(self.files.root()).ok()?)?;
                Some(TreeFile::new(module, path))
            })
            .collect();
        files.sort();
        if let Some(file) = files.iter().find(|file| file.is_unnamable()) {
            return Err(file.report_unnamable());
        }
        self.taken.refuse(&files, terminal)?;
        Ok(files.into_iter().map(TreeFile::into_module).collect())
    }
}
