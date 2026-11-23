use super::exported_task::ExportedTask;

#[derive(Debug, Default)]
pub enum ExportedTasks {
    #[default]
    Unwritten,
    Written(Vec<ExportedTask>),
}

impl ExportedTasks {
    pub fn is_written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    pub fn names(&self) -> Vec<String> {
        match self {
            Self::Unwritten => Vec::new(),
            Self::Written(tasks) => tasks.iter().map(|task| task.name.clone()).collect(),
        }
    }

    pub fn into_tasks(self) -> Vec<ExportedTask> {
        match self {
            Self::Unwritten => Vec::new(),
            Self::Written(tasks) => tasks,
        }
    }

    pub fn written(&mut self) -> &mut Vec<ExportedTask> {
        match self {
            Self::Unwritten => {
                *self = Self::Written(Vec::new());
                self.written()
            }
            Self::Written(tasks) => tasks,
        }
    }
}
