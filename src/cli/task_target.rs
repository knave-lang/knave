use std::path::Path;

use super::build_route::BuildRoute;
use super::task_name::TaskName;

pub trait TaskTarget {
    fn name(&self) -> &TaskName;
    fn binary(&self) -> &Path;
    fn route(&self) -> Box<dyn BuildRoute + '_>;
}
