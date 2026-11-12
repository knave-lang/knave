use std::path::{Path, PathBuf};

use knave_expand::diagnostic::DiagnosticCode;

use crate::subject_program::SubjectProgram;
use crate::top_level_definitions::TopLevelDefinitions;
use crate::verdict::{RejectingTool, Rejection};

pub struct KnaveStandardError<'a> {
    program: &'a SubjectProgram,
    definitions: TopLevelDefinitions,
    expanded: TopLevelDefinitions,
}

struct Heading {
    code: Option<DiagnosticCode>,
}

struct Location {
    file: PathBuf,
    line: usize,
    column: usize,
}

impl<'a> KnaveStandardError<'a> {
    const ERROR: &'static str = "error";
    const LOCATION_ARROW: &'static str = "--> ";

    pub fn of(program: &'a SubjectProgram, expanded: TopLevelDefinitions) -> Self {
        Self {
            program,
            definitions: program.definitions(),
            expanded,
        }
    }

    pub fn rejections(&self, standard_error: &str) -> Vec<Rejection> {
        let mut rejections = Vec::new();
        let mut heading: Option<Heading> = None;
        for line in standard_error.lines() {
            if let Some(found) = Self::heading(line) {
                heading = Some(found);
            } else if let (Some(open), Some(location)) = (&heading, Self::location(line)) {
                rejections.push(self.rejection(open.code.clone(), location));
                heading = None;
            }
        }
        rejections
    }

    fn heading(line: &str) -> Option<Heading> {
        let rest = line.strip_prefix(Self::ERROR)?;
        let code = match rest.strip_prefix('[') {
            Some(bracketed) => Some(DiagnosticCode::from_text(
                bracketed.split(']').next()?.to_string(),
            )),
            None if rest.starts_with(':') => None,
            None => return None,
        };
        Some(Heading { code })
    }

    fn location(line: &str) -> Option<Location> {
        let (_, place) = line.split_once(Self::LOCATION_ARROW)?;
        let mut parts = place.trim().rsplitn(3, ':');
        let column = parts.next()?.parse().ok()?;
        let line = parts.next()?.parse().ok()?;
        Some(Location {
            file: PathBuf::from(parts.next()?),
            line,
            column,
        })
    }

    fn rejection(&self, code: Option<DiagnosticCode>, location: Location) -> Rejection {
        let text = self.program.text_of(&location.file).unwrap_or_default();
        let offset = Self::offset(text, location.line, location.column);
        let function = self.function_at(&location.file, text, offset);
        Rejection {
            tool: RejectingTool::Knave,
            code,
            reached_from_main: function
                .as_deref()
                .is_none_or(|name| self.expanded.reaches(name)),
            function,
            span: offset..offset,
            file: location.file,
        }
    }

    fn function_at(&self, file: &Path, text: &str, offset: usize) -> Option<String> {
        if file == self.program.entry {
            self.definitions.name_at(offset).map(str::to_string)
        } else {
            TopLevelDefinitions::of_text(text)
                .name_at(offset)
                .map(str::to_string)
        }
    }

    fn offset(text: &str, line: usize, column: usize) -> usize {
        let line_start: usize = text
            .split_inclusive('\n')
            .take(line.saturating_sub(1))
            .map(str::len)
            .sum();
        let within_line = text[line_start..]
            .char_indices()
            .nth(column.saturating_sub(1))
            .map_or(0, |(index, _)| index);
        line_start + within_line
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::KnaveStandardError;
    use crate::subject_program::SubjectProgram;

    struct StandardErrorFixture {
        program: SubjectProgram,
    }

    impl StandardErrorFixture {
        const MOVED: &'static str =
            "(defn main ()\n  (local v (vec 1))\n  (local w v)\n  (println (len v) (len w)))\n";
        const REPORT: &'static str =
            "error[L0501]: use of moved value `v`\n --> moved.knv:4:17\n  |\n3 |   (local w v)\n";

        fn new() -> Self {
            Self {
                program: SubjectProgram::of_entry(
                    "moved".to_string(),
                    PathBuf::from("moved.knv"),
                    Self::MOVED.to_string(),
                ),
            }
        }
    }

    #[test]
    fn reads_each_error_code_with_its_file_offset_and_function() {
        let fixture = StandardErrorFixture::new();
        let rejections = KnaveStandardError::of(&fixture.program, fixture.program.definitions())
            .rejections(StandardErrorFixture::REPORT);
        assert_eq!(rejections.len(), 1);
        assert_eq!(
            rejections[0].code.as_ref().map(|code| code.as_str()),
            Some("L0501")
        );
        assert_eq!(rejections[0].file, PathBuf::from("moved.knv"));
        assert_eq!(
            &StandardErrorFixture::MOVED[rejections[0].span.start..][..1],
            "v"
        );
        assert_eq!(rejections[0].function.as_deref(), Some("main"));
    }

    #[test]
    fn skips_warnings_and_keeps_an_error_without_a_code_as_knave_s() {
        let fixture = StandardErrorFixture::new();
        let report = "warning[L0434]: unreachable\n --> moved.knv:1:1\nerror: cannot read\n --> moved.knv:1:2\n";
        let rejections = KnaveStandardError::of(&fixture.program, fixture.program.definitions())
            .rejections(report);
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].code, None);
        assert!(!rejections[0].is_rustc());
    }
}
