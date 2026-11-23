use crate::reader::StringLiteral;

pub struct LenderArguments<'l> {
    library: &'l str,
    fetched: bool,
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
            fetched: false,
            exported_tasks,
        }
    }

    pub fn fetched(library: &'l str, exported_tasks: &'l [String]) -> Self {
        Self {
            fetched: true,
            ..Self::local(library, exported_tasks)
        }
    }

    pub fn export_check(&self) -> String {
        let key = if self.fetched {
            Self::GIT_LIBRARY
        } else {
            Self::LIBRARY
        };
        format!(" {key} {}", StringLiteral::quoted(self.library))
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
