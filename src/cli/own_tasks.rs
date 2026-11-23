use std::rc::Rc;

use super::entry_tree::EntryTree;
use super::exit_status::ExitStatus;
use super::located_project::LocatedProject;
use super::project_task::ProjectTask;
use super::task_catalog::TaskCatalog;
use super::task_invocation::TaskInvocation;
use super::task_name::TaskName;
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
        let catalog = TaskCatalog::load(&tree, terminal, "")?;
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

    pub fn defines(&self, task: &str) -> bool {
        self.names.iter().any(|name| name.as_ref() == task)
    }

    pub fn listing(&self, terminal: &mut Terminal) -> Result<String, ExitStatus> {
        match &self.catalog {
            Some(catalog) => catalog.listing(terminal, ""),
            None => Ok(String::new()),
        }
    }

    pub fn run(
        &self,
        terminal: &mut Terminal,
        task: &str,
        invocation: TaskInvocation<'_>,
    ) -> Result<ExitStatus, ExitStatus> {
        let catalog = self
            .catalog
            .as_ref()
            .expect("a project that defines a task has a task catalog");
        let target = ProjectTask::new(self.project(), TaskName::new(task.to_string()));
        invocation.run(terminal, catalog, &target)
    }
}
