#[derive(Clone, Copy)]
pub struct TaskLayout {
    pub exports_tasks: bool,
    pub tasks_folder: bool,
    pub entry_left_out: bool,
}
