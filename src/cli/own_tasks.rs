use std::rc::Rc;

use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::located_project::LocatedProject;
use super::project_task::ProjectTask;
use super::task_catalog::TaskCatalog;
use super::task_invocation::TaskInvocation;
use super::task_name::TaskName;
use super::task_runners::TaskRunners;
use super::terminal::Terminal;

pub struct OwnTasks {
    tree: EntryTree,
    catalog: Option<TaskCatalog>,
    names: Vec<Rc<str>>,
}

impl OwnTasks {
    pub fn load(terminal: &mut Terminal, project: LocatedProject) -> Result<Self, ExitStatus> {
        let has_tasks = project.task_layout().has_tasks();
        let tree = EntryTree::of_tasks(project);
        if !has_tasks {
            return Ok(Self {
                tree,
                catalog: None,
                names: Vec::new(),
            });
        }
        let catalog = TaskCatalog::load(&tree, terminal, None)?;
        let names = catalog
            .names(terminal)?
            .iter()
            .map(|name| Rc::from(name.as_str()))
            .collect();
        catalog.exports()?;
        Ok(Self {
            tree,
            catalog: Some(catalog),
            names,
        })
    }

    pub fn names(&self) -> &[Rc<str>] {
        &self.names
    }

    pub fn project(&self) -> &LocatedProject {
        self.tree
            .project()
            .expect("the task tree belongs to a project")
    }

    pub fn run(
        &self,
        terminal: &mut Terminal,
        task: &str,
        invocation: TaskInvocation<'_>,
    ) -> Option<Result<ExitStatus, ExitStatus>> {
        let catalog = self.catalog.as_ref()?;
        if !self.names.iter().any(|name| name.as_ref() == task) {
            return None;
        }
        let task = ProjectTask::new(self.project(), TaskName::new(task.to_string()));
        let built = TaskRunners::new(catalog).build(terminal, &task, invocation.profile);
        Some(built.map(|binary| {
            FinishedProgram::run_in(&binary, invocation.arguments, task.root()).status()
        }))
    }
}
