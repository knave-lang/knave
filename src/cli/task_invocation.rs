use crate::build::BuildProfile;

#[derive(Clone, Copy)]
pub struct TaskInvocation<'a> {
    pub arguments: &'a [String],
    pub profile: BuildProfile,
}
