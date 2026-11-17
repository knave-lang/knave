use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use anstream::AutoStream;

use super::diagnostic_output::JsonDiagnostics;
use super::exit_status::ExitStatus;
use super::finished_program::FinishedProgram;
use super::json_replies::{JsonObject, JsonReplies};
use super::painted_program::PaintedProgram;
use super::streamed_program::StreamedProgram;
use super::terminal::Terminal;
use super::token_colors::TokenColors;
use super::value_marker::ValueMarker;
use super::written_printer::WrittenPrinter;
use crate::diagnostic::Diagnostics;
use crate::expand::{ExpansionSession, ResultPrinter};

pub trait EvaluationOutput {
    fn attach(&self, terminal: &mut Terminal);
    fn printer(&self, session: &mut ExpansionSession<'_>) -> Result<ResultPrinter, Diagnostics>;
    fn run(&self, binary: &Path, printed_forms: Vec<usize>) -> FinishedProgram;
    fn finish(&self, status: ExitStatus);
}

pub enum PrintedEvaluation {
    Plain,
    Painted(ValueMarker),
}

pub struct JsonEvaluation {
    replies: Arc<JsonReplies>,
    marker: ValueMarker,
}

impl PrintedEvaluation {
    const PRINTER: &'static str = "print-result";
    const PRINTER_SOURCE: &'static str = "<result printer>";
    const PRINTER_TEXT: &'static str = "(fn ((value (ref _))) (prn value))";
    const PRINTER_BOUND: &'static str = "prn";

    fn deciding(colors: TokenColors) -> Self {
        match colors {
            TokenColors::Painted => Self::Painted(ValueMarker::random()),
            TokenColors::Plain => Self::Plain,
        }
    }

    pub fn for_standard_output() -> Self {
        Self::deciding(TokenColors::deciding(AutoStream::choice(&io::stdout())))
    }

    fn plain_printer(session: &mut ExpansionSession<'_>) -> Result<ResultPrinter, Diagnostics> {
        WrittenPrinter::new(Self::PRINTER, Self::PRINTER_SOURCE, Self::PRINTER_BOUND)
            .build(session, Self::PRINTER_TEXT)
    }
}

impl EvaluationOutput for PrintedEvaluation {
    fn attach(&self, _terminal: &mut Terminal) {}

    fn printer(&self, session: &mut ExpansionSession<'_>) -> Result<ResultPrinter, Diagnostics> {
        match self {
            Self::Plain => Self::plain_printer(session),
            Self::Painted(marker) => marker.printer(session),
        }
    }

    fn run(&self, binary: &Path, printed_forms: Vec<usize>) -> FinishedProgram {
        match self {
            Self::Plain => FinishedProgram::run(binary),
            Self::Painted(marker) => {
                PaintedProgram::new(marker.stream(printed_forms)).run(binary, io::stdout().lock())
            }
        }
    }

    fn finish(&self, _status: ExitStatus) {}
}

impl JsonEvaluation {
    const STATUS: &'static str = "status";
    const DONE: &'static str = "done";
    const EXIT: &'static str = "exit";

    pub fn new(id: Option<String>) -> Self {
        Self {
            replies: Arc::new(JsonReplies::new(id)),
            marker: ValueMarker::random(),
        }
    }
}

impl EvaluationOutput for JsonEvaluation {
    fn attach(&self, terminal: &mut Terminal) {
        let output = Rc::new(JsonDiagnostics::new(Arc::clone(&self.replies)));
        terminal.configure(|settings| settings.rerouted_to(output));
    }

    fn printer(&self, session: &mut ExpansionSession<'_>) -> Result<ResultPrinter, Diagnostics> {
        self.marker.printer(session)
    }

    fn run(&self, binary: &Path, printed_forms: Vec<usize>) -> FinishedProgram {
        let output = self.marker.stream(printed_forms);
        StreamedProgram::new(Arc::clone(&self.replies)).run(binary, output)
    }

    fn finish(&self, status: ExitStatus) {
        if status == ExitStatus::UsageError {
            return;
        }
        let done = JsonObject::new(Self::STATUS, Self::DONE).with(Self::EXIT, status.code());
        let _ = self.replies.send(done);
    }
}
