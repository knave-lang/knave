use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::exit_status::ExitStatus;
use super::standard_error::StandardError;
use crate::language::Language;

pub struct DocFolder {
    path: PathBuf,
}

pub struct WritableFolder {
    path: PathBuf,
    preparation: Preparation,
}

pub struct WrittenPage {
    pub module: String,
    pub text: String,
}

impl WrittenPage {
    const EXTENSION: &'static str = "md";

    pub fn file_name(module: &str) -> String {
        format!("{module}.{}", Self::EXTENSION)
    }
}

enum Preparation {
    Create,
    Keep,
    Empty,
}

impl DocFolder {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn writable(self) -> Result<WritableFolder, ExitStatus> {
        match self.preparation() {
            Ok(Some(preparation)) => Ok(WritableFolder {
                path: self.path,
                preparation,
            }),
            Ok(None) => Err(self.report_foreign_files()),
            Err(error) => Err(WritableFolder::report_unusable(&self.path, &error)),
        }
    }

    fn preparation(&self) -> io::Result<Option<Preparation>> {
        let mut entries = match fs::read_dir(&self.path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Some(Preparation::Create));
            }
            read => read?,
        };
        if entries.next().is_none() {
            Ok(Some(Preparation::Keep))
        } else if WritableFolder::marker(&self.path).is_file() {
            Ok(Some(Preparation::Empty))
        } else {
            Ok(None)
        }
    }

    fn report_foreign_files(&self) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: `{folder}` holds files that `{} doc --write` did not write\n\
             help: name an empty folder with `--output`, or empty `{folder}`",
            Language::NAME,
            folder = self.path.display()
        ));
        ExitStatus::UsageError
    }
}

impl WritableFolder {
    pub fn replace_pages(&self, pages: &[WrittenPage]) -> ExitStatus {
        match self.prepared().and_then(|()| self.write_pages(pages)) {
            Ok(()) => ExitStatus::Success,
            Err(error) => Self::report_unusable(&self.path, &error),
        }
    }

    fn prepared(&self) -> io::Result<()> {
        match self.preparation {
            Preparation::Create => fs::create_dir_all(&self.path),
            Preparation::Keep => Ok(()),
            Preparation::Empty => self.remove_entries(),
        }
    }

    fn remove_entries(&self) -> io::Result<()> {
        for entry in fs::read_dir(&self.path)? {
            let path = entry?.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    fn write_pages(&self, pages: &[WrittenPage]) -> io::Result<()> {
        fs::write(Self::marker(&self.path), Self::marker_text())?;
        pages.iter().try_for_each(|page| self.write_page(page))
    }

    fn write_page(&self, page: &WrittenPage) -> io::Result<()> {
        let path = self.path.join(WrittenPage::file_name(&page.module));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &page.text)
    }

    fn marker_text() -> String {
        format!(
            "`{} doc --write` wrote this folder, and empties it before it writes again.\n",
            Language::NAME
        )
    }

    fn marker(folder: &Path) -> PathBuf {
        folder.join(format!(".{}-doc", Language::NAME))
    }

    fn report_unusable(folder: &Path, error: &io::Error) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: cannot write the pages into `{}`: {error}",
            folder.display()
        ));
        ExitStatus::Environment
    }
}
