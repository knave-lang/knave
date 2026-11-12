use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::finding::Finding;
use crate::finding_list::FindingList;
use crate::subject_program::SubjectProgram;

pub struct FindingsDirectory {
    path: PathBuf,
}

impl FindingsDirectory {
    const LIST_FILE: &'static str = "findings.txt";
    const PROGRAM_EXTENSION: &'static str = "knv";
    const REDUCED: &'static str = "reduced";

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn record(&self, findings: &[Finding]) -> io::Result<FindingList> {
        fs::create_dir_all(&self.path)?;
        self.remove_earlier_programs()?;
        for finding in findings {
            finding.write_program(&self.path)?;
        }
        let list = FindingList::of(findings);
        list.write(&self.path.join(Self::LIST_FILE))?;
        Ok(list)
    }

    pub fn record_reduced(&self, name: &str, program: &SubjectProgram) -> io::Result<()> {
        let path = self.path.join(format!(
            "{name}.{}.{}",
            Self::REDUCED,
            Self::PROGRAM_EXTENSION
        ));
        fs::write(path, program.entry_text())
    }

    fn remove_earlier_programs(&self) -> io::Result<()> {
        for entry in fs::read_dir(&self.path)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == Self::PROGRAM_EXTENSION)
            {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::FindingsDirectory;
    use crate::scratch_directory::ScratchDirectory;

    #[test]
    fn replaces_the_programs_of_an_earlier_run_and_keeps_other_files() {
        let scratch = ScratchDirectory::create().expect("a scratch directory");
        let earlier = scratch.path().join("seed-9.knv");
        let note = scratch.path().join("note.txt");
        fs::write(&earlier, "(defn main () 1)").expect("writable");
        fs::write(&note, "kept").expect("writable");
        let list = FindingsDirectory::at(scratch.path().to_path_buf())
            .record(&[])
            .expect("the directory records");
        assert!(!earlier.exists());
        assert!(note.exists());
        assert_eq!(list.text(), "");
        assert!(scratch.path().join("findings.txt").exists());
    }
}
