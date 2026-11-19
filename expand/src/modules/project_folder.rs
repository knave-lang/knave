#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectFolder {
    Tasks,
    Tests,
    Source,
}

impl ProjectFolder {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tasks => "tasks",
            Self::Tests => "tests",
            Self::Source => "source",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectFolder;

    #[test]
    fn names_each_folder_as_the_prelude_reads_it() {
        let names = [
            ProjectFolder::Tasks,
            ProjectFolder::Tests,
            ProjectFolder::Source,
        ]
        .map(ProjectFolder::name);
        assert_eq!(names, ["tasks", "tests", "source"]);
    }
}
