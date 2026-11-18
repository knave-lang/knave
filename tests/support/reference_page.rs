use std::fs;
use std::path::PathBuf;
use std::thread;

use knave::cli::ReferencePages;

use super::Fixture;
use super::example_folder::ExampleFolder;
use super::example_layout::ExampleLayout;

pub struct ReferencePage {
    name: String,
    text: String,
}

pub struct PreludePages {
    folder: ExampleFolder,
}

struct ReferenceExample<'p> {
    place: String,
    code: Option<&'p str>,
    shown_output: Option<&'p str>,
}

struct PreludeLink<'p> {
    target: &'p str,
    module: &'p str,
    anchor: &'p str,
}

impl ReferencePage {
    const EXAMPLE_FENCE: &str = "```knave\n";
    const OUTPUT_FENCE: &str = "```text\n";
    const CLOSING_FENCE: &str = "```\n";
    const FENCE_START: &str = "```";
    const HEADING_START: char = '#';
    const ANCHOR_SEPARATOR: char = '-';
    const ANCHOR_PUNCTUATION: &str = " -_";
    const PRELUDE_LINK_START: &str = "](/docs/prelude/";
    const LINK_END: char = ')';
    const MAXIMUM_PARALLEL_BUILDS: usize = 4;

    pub fn written() -> Vec<Self> {
        let mut paths: Vec<PathBuf> = fs::read_dir(ReferencePages::FOLDER)
            .expect("the reference folder is readable")
            .map(|entry| entry.expect("a reference entry is readable").path())
            .collect();
        paths.sort();
        paths.iter().map(Self::load).collect()
    }

    fn load(path: &PathBuf) -> Self {
        let name = path.file_stem().expect("a page has a name");
        let text = fs::read_to_string(path).expect("a reference page is readable");
        Self::new(&name.to_string_lossy(), &text)
    }

    pub fn new(name: &str, text: &str) -> Self {
        let name = name.to_string();
        let text = text.to_string();
        Self { name, text }
    }

    pub fn output_failures(&self) -> Vec<String> {
        let examples = self.examples();
        examples
            .chunks(Self::MAXIMUM_PARALLEL_BUILDS)
            .flat_map(|batch| self.failures_of_batch(batch))
            .collect()
    }

    pub fn mislaid_examples(&self) -> Vec<String> {
        self.examples()
            .iter()
            .filter_map(ReferenceExample::mislaid)
            .collect()
    }

    pub fn names_word(&self, word: &str) -> bool {
        self.text
            .split(|character: char| !character.is_alphanumeric() && character != '_')
            .any(|written| written == word)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn has_heading_anchor(&self, anchor: &str) -> bool {
        self.text
            .lines()
            .filter_map(|line| line.strip_prefix(Self::HEADING_START))
            .map(|heading| Self::anchor_of(heading.trim_start_matches(Self::HEADING_START)))
            .any(|heading_anchor| heading_anchor == anchor)
    }

    fn anchor_of(heading: &str) -> String {
        heading
            .trim()
            .to_lowercase()
            .chars()
            .filter(|character| {
                character.is_alphanumeric() || Self::ANCHOR_PUNCTUATION.contains(*character)
            })
            .map(|character| match character {
                ' ' => Self::ANCHOR_SEPARATOR,
                kept => kept,
            })
            .collect()
    }

    pub fn broken_prelude_links(&self, prelude: &PreludePages) -> Vec<String> {
        self.prelude_links()
            .filter(|link| !prelude.defines(link))
            .map(|link| {
                format!(
                    "{} links to {link}, which no prelude page defines",
                    self.name
                )
            })
            .collect()
    }

    fn failures_of_batch(&self, batch: &[ReferenceExample]) -> Vec<String> {
        thread::scope(|scope| {
            let runs: Vec<_> = batch
                .iter()
                .map(|example| scope.spawn(move || self.failure_of(example)))
                .collect();
            runs.into_iter()
                .filter_map(|run| run.join().expect("an example run ends"))
                .collect()
        })
    }

    fn failure_of(&self, example: &ReferenceExample) -> Option<String> {
        let Some(code) = example.code else {
            return Some(example.unclosed_failure());
        };
        let file = format!("{}.knv", example.place.replace(' ', "-"));
        let path = PathBuf::from(Fixture::FIXTURE_ROOT).join(file);
        let printed = Fixture::of_text(&path, code.to_string()).run_report();
        example.output_failure(&printed)
    }

    fn examples(&self) -> Vec<ReferenceExample<'_>> {
        self.text
            .split(Self::EXAMPLE_FENCE)
            .skip(1)
            .enumerate()
            .map(|(index, rest)| self.example_at(index, rest))
            .collect()
    }

    fn example_at<'p>(&self, index: usize, rest: &'p str) -> ReferenceExample<'p> {
        let place = format!("{} example {index}", self.name);
        let closed = Self::closing_fence_offset(rest).map(|end| rest.split_at(end));
        let shown_output = closed.and_then(|(_, after_code)| Self::shown_output_of(after_code));
        ReferenceExample {
            place,
            code: closed.map(|(code, _)| code),
            shown_output,
        }
    }

    fn closing_fence_offset(rest: &str) -> Option<usize> {
        let mut offset = 0;
        for line in rest.split_inclusive('\n') {
            if line == Self::CLOSING_FENCE {
                return Some(offset);
            }
            if line.starts_with(Self::FENCE_START) {
                return None;
            }
            offset += line.len();
        }
        None
    }

    fn shown_output_of(after_code: &str) -> Option<&str> {
        after_code
            .strip_prefix(Self::CLOSING_FENCE)?
            .trim_start_matches('\n')
            .strip_prefix(Self::OUTPUT_FENCE)
            .and_then(|output| output.split(Self::CLOSING_FENCE).next())
    }

    fn prelude_links(&self) -> impl Iterator<Item = PreludeLink<'_>> {
        self.text
            .split(Self::PRELUDE_LINK_START)
            .skip(1)
            .filter_map(|rest| rest.split(Self::LINK_END).next())
            .map(PreludeLink::parse)
    }
}

impl ReferenceExample<'_> {
    fn output_failure(&self, printed: &str) -> Option<String> {
        match self.shown_output {
            Some(shown) if shown == printed => None,
            Some(shown) => Some(format!(
                "{} shows this output:\n{shown}\nbut prints:\n{printed}",
                self.place
            )),
            None => Some(format!(
                "{} shows no output: put a `text` block of what it prints after it",
                self.place
            )),
        }
    }

    fn unclosed_failure(&self) -> String {
        format!(
            "{} has no closing fence: end the example with a line that holds only ```",
            self.place
        )
    }

    fn mislaid(&self) -> Option<String> {
        let Some(code) = self.code else {
            return Some(self.unclosed_failure());
        };
        match ExampleLayout::of_single_file(code).laid_out() {
            Ok(laid_out) if laid_out == code => None,
            Ok(laid_out) => Some(format!(
                "{} is not laid out as `knave fmt` lays it out. Replace the block with:\n\n{laid_out}",
                self.place
            )),
            Err(refusal) => Some(format!("{}: {refusal}", self.place)),
        }
    }
}

impl<'p> PreludeLink<'p> {
    const ANCHOR_START: char = '#';
    const PATH_SEPARATOR: char = '/';

    fn parse(target: &'p str) -> Self {
        let (path, anchor) = target
            .split_once(Self::ANCHOR_START)
            .unwrap_or((target, ""));
        let module = path.trim_end_matches(Self::PATH_SEPARATOR);
        Self {
            target,
            module,
            anchor,
        }
    }
}

impl std::fmt::Display for PreludeLink<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "/docs/prelude/{}", self.target)
    }
}

impl PreludePages {
    const COMMAND: [&str; 3] = ["doc", "--write", "--prelude"];
    const PAGES_FOLDER: &str = "doc/prelude";
    const ROOT_PAGE: &str = "doc/prelude.md";

    pub fn written() -> Self {
        let folder = ExampleFolder::holding([]);
        folder.run(&Self::COMMAND.map(String::from), "");
        Self { folder }
    }

    fn defines(&self, link: &PreludeLink) -> bool {
        let anchor = format!("<a id=\"{}\"></a>", link.anchor);
        self.folder
            .contents(&Self::page_path(link))
            .is_some_and(|page| page.contains(&anchor))
    }

    fn page_path(link: &PreludeLink) -> String {
        match link.module {
            "" => Self::ROOT_PAGE.to_string(),
            module => format!("{}/{module}.md", Self::PAGES_FOLDER),
        }
    }
}
