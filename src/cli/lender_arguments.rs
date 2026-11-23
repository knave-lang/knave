use crate::reader::StringLiteral;

pub struct LenderArguments<'l> {
    library: &'l str,
    library_key: &'static str,
    exported_tasks: &'l [String],
}

impl<'l> LenderArguments<'l> {
    const LIBRARY: &'static str = ":library";
    const GIT_LIBRARY: &'static str = ":git-library";
    const EXPORTED: &'static str = ":exported";
    const QUALIFIED: &'static str = ":qualified";

    pub fn local(library: &'l str, exported_tasks: &'l [String]) -> Self {
        Self {
            library,
            library_key: Self::LIBRARY,
            exported_tasks,
        }
    }

    pub fn fetched(library: &'l str, exported_tasks: &'l [String]) -> Self {
        Self {
            library_key: Self::GIT_LIBRARY,
            ..Self::local(library, exported_tasks)
        }
    }

    pub fn export_check(&self) -> String {
        format!(
            " {} {}",
            self.library_key,
            StringLiteral::quoted(self.library)
        )
    }

    pub fn listing(&self, qualified: impl Fn(&str) -> bool) -> String {
        let library = format!(" {} {}", Self::LIBRARY, StringLiteral::quoted(self.library));
        let rows = self.exported_tasks.iter().map(|task| {
            let exported = format!(" {} {}", Self::EXPORTED, StringLiteral::quoted(task));
            if qualified(task) {
                format!(
                    "{exported} {} {}",
                    Self::QUALIFIED,
                    StringLiteral::quoted(task)
                )
            } else {
                exported
            }
        });
        std::iter::once(library).chain(rows).collect()
    }
}
