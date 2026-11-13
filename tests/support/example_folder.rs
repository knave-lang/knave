use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use knave::language::Language;

pub struct ExampleFolder {
    path: PathBuf,
}

pub struct ExampleFile {
    pub path: String,
    pub text: String,
}

impl ExampleFolder {
    const COMPILER: &str = env!("CARGO_BIN_EXE_knave");
    const PLAIN_OUTPUT: [&str; 2] = ["--color", "never"];
    const CACHE_HOME_VARIABLE: &str = "XDG_CACHE_HOME";
    const CACHE_FOLDER: &str = ".cache";

    pub fn holding<'f>(files: impl IntoIterator<Item = &'f ExampleFile>) -> Self {
        static NEXT_EXAMPLE: AtomicUsize = AtomicUsize::new(0);
        let example = NEXT_EXAMPLE.fetch_add(1, Ordering::Relaxed);
        let folder = Self {
            path: std::env::temp_dir().join(format!(
                "{}-example-{}-{example}",
                Language::NAME,
                std::process::id()
            )),
        };
        fs::create_dir_all(&folder.path).expect("the example folder is writable");
        for file in files {
            folder.write(file);
        }
        folder
    }

    pub fn run(&self, command: &[String], input: &str) -> Output {
        let mut child = Command::new(Self::COMPILER)
            .args(Self::PLAIN_OUTPUT)
            .args(command)
            .current_dir(&self.path)
            .env(
                Self::CACHE_HOME_VARIABLE,
                self.path.join(Self::CACHE_FOLDER),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the compiler starts");
        child
            .stdin
            .take()
            .expect("the compiler's input is piped")
            .write_all(input.as_bytes())
            .expect("the compiler reads its input");
        child.wait_with_output().expect("the compiler finishes")
    }

    pub fn text_of(&self, file: &ExampleFile) -> String {
        fs::read_to_string(self.path.join(&file.path)).expect("the example file is readable")
    }

    fn write(&self, file: &ExampleFile) {
        let path = self.path.join(&file.path);
        fs::create_dir_all(path.parent().expect("a file lies in a folder"))
            .expect("the example folder is writable");
        fs::write(path, &file.text).expect("the example file is writable");
    }
}

impl Drop for ExampleFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
