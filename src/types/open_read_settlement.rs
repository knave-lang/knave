use rustc_hash::{FxHashMap, FxHashSet};

use super::check_round::{CheckRound, RoundGrowth};
use super::display::TypeDisplay;
use super::failure_kind::FailureKind;
use super::field_reader::FieldReader;
use super::field_readers::FieldReaders;
use super::graph::DefinitionGraph;
use super::inferred_bound_problems::InferredBoundProblems;
use super::pinned_owners::PinnedOwners;
use super::pinning::InstantiationReport;
use super::reaching_reads::ReachingReads;
use super::reader_shape::ReaderShape;
use super::round_failure::RoundFailure;
use super::settled_read::SettledRead;
use super::settlement_failure::SettlementFailure;
use super::settlement_verdicts::SettlementVerdicts;
use super::ty::Type;
use super::typed::TypedProgram;
use super::written_refusal::{ReadRefusal, UseRefusal, WrittenRefusal};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;
use crate::expand::Program;
use crate::source::Span;

pub(super) struct OpenReadSettlement<'p> {
    program: &'p Program,
    graph: &'p DefinitionGraph<'p>,
    settled: &'p FxHashMap<Span, SettledRead>,
}

enum OpenReadOutcome {
    Settled(FxHashMap<Span, SettledRead>),
    Unsettled(SettlementVerdicts),
}

struct ReadTally {
    settled: FxHashMap<Span, SettledRead>,
    pinned: FxHashMap<Span, BindingId>,
    waiting_reads: FxHashSet<Span>,
    agreed: FxHashMap<Span, Type>,
    verdicts: SettlementVerdicts,
}

enum ReaderRead {
    Agreed(Type),
    Pinned(BindingId),
    Unreached,
    Waiting(Diagnostic),
    Refused(Diagnostic),
}

impl<'p> OpenReadSettlement<'p> {
    pub(super) fn new(
        program: &'p Program,
        graph: &'p DefinitionGraph<'p>,
        settled: &'p FxHashMap<Span, SettledRead>,
    ) -> Self {
        Self {
            program,
            graph,
            settled,
        }
    }

    pub(super) fn after(&self, round: CheckRound) -> CheckRound {
        match round {
            CheckRound::Checked(typed) => self.round(typed),
            CheckRound::FailedWithProgram(failure, typed) => self.after_failure(typed, failure),
            ended => ended,
        }
    }

    fn round(&self, typed: Box<TypedProgram>) -> CheckRound {
        match self.settle(&typed) {
            OpenReadOutcome::Settled(reads) => Self::grown(&typed, reads),
            OpenReadOutcome::Unsettled(verdicts) => match verdicts.into_failure() {
                Some(SettlementFailure::Refused(failure) | SettlementFailure::Waiting(failure)) => {
                    CheckRound::Failed(failure)
                }
                None => CheckRound::Checked(typed),
            },
        }
    }

    fn after_failure(&self, typed: Box<TypedProgram>, failure: RoundFailure) -> CheckRound {
        match self.settle(&typed) {
            OpenReadOutcome::Settled(reads) => Self::grown(&typed, reads),
            OpenReadOutcome::Unsettled(verdicts) => CheckRound::Failed(failure.ended(verdicts)),
        }
    }

    fn grown(typed: &TypedProgram, reads: FxHashMap<Span, SettledRead>) -> CheckRound {
        CheckRound::Grown(RoundGrowth::SettledReads(reads), typed.checking)
    }

    fn settle(&self, typed: &TypedProgram) -> OpenReadOutcome {
        let readers = self.open_readers(typed);
        if readers.is_empty() {
            return OpenReadOutcome::Unsettled(SettlementVerdicts::default());
        }
        let report = InstantiationReport::of(self.program, self.graph, typed);
        let pinned = PinnedOwners::new(&report, typed, self.graph).of(&readers);
        let mut tally = ReadTally::new(self.settled.clone());
        for reader in readers {
            let read = match pinned.get(&PinnedOwners::key(&reader)) {
                Some(owner) => ReaderRead::Pinned(*owner),
                None => Self::reader_read(&report, typed, &reader),
            };
            tally.add(&reader, read);
        }
        let display = TypeDisplay::new(&typed.bindings, &typed.generics);
        tally.outcome(self.settled.len(), &display)
    }

    fn open_readers(&self, typed: &TypedProgram) -> Vec<FieldReader> {
        let readers = FieldReaders::of(typed).into_iter();
        readers
            .filter_map(|reader| self.unsettled(reader))
            .collect()
    }

    fn unsettled(&self, mut reader: FieldReader) -> Option<FieldReader> {
        (reader.inferred_at).retain_unsettled(self.settled);
        reader.inferred_at.is_inferred().then_some(reader)
    }

    fn reader_read(
        report: &InstantiationReport,
        typed: &TypedProgram,
        reader: &FieldReader,
    ) -> ReaderRead {
        let Some(read) = reader.inferred_at.first() else {
            return ReaderRead::Unreached;
        };
        let problems = InferredBoundProblems::new(&reader.field, read);
        let Ok(reads) = ReachingReads::new(report, typed).of(reader) else {
            let open_struct = WrittenRefusal::Read(ReadRefusal::OpenStruct);
            return ReaderRead::Waiting(problems.unkept(open_struct, None));
        };
        match reads.distinct().as_slice() {
            [] => ReaderRead::Unreached,
            [_] if ReaderShape::new(typed).changes_a_field(reader) => {
                let changes = WrittenRefusal::Use(UseRefusal::ChangesField);
                ReaderRead::Refused(problems.unkept(changes, None))
            }
            [reached] => ReaderRead::Agreed(reached.read.read.clone()),
            _ if reads.reads_function() => {
                let function = WrittenRefusal::Read(ReadRefusal::Function);
                ReaderRead::Refused(problems.unkept(function, None))
            }
            _ => ReaderRead::Refused(problems.differing_reads(&reads.shown(typed))),
        }
    }
}

impl ReadTally {
    fn new(settled: FxHashMap<Span, SettledRead>) -> Self {
        Self {
            settled,
            pinned: FxHashMap::default(),
            waiting_reads: FxHashSet::default(),
            agreed: FxHashMap::default(),
            verdicts: SettlementVerdicts::default(),
        }
    }

    fn add(&mut self, reader: &FieldReader, read: ReaderRead) {
        let spans = reader.inferred_at.spans();
        match read {
            ReaderRead::Agreed(read) => {
                (self.agreed).extend(spans.map(|span| (span, read.clone())));
            }
            ReaderRead::Pinned(owner) => self.pinned.extend(spans.map(|span| (span, owner))),
            ReaderRead::Unreached => {}
            ReaderRead::Waiting(refusal) => {
                self.waiting_reads.extend(spans);
                self.verdicts.wait(refusal)
            }
            ReaderRead::Refused(refusal) => self.verdicts.refuse(spans, refusal),
        }
    }

    fn outcome(mut self, known: usize, display: &TypeDisplay) -> OpenReadOutcome {
        self.settle_pinned();
        let agreed = std::mem::take(&mut self.agreed);
        for (span, read) in &agreed {
            self.verdicts.agree(*span, display.show(read));
        }
        (self.settled).extend(
            agreed
                .into_iter()
                .map(|(span, read)| (span, SettledRead::Read(read))),
        );
        let refused = self.verdicts.failure_kind() == Some(FailureKind::Refused);
        if !refused && self.settled.len() > known {
            OpenReadOutcome::Settled(self.settled)
        } else {
            OpenReadOutcome::Unsettled(self.verdicts)
        }
    }

    fn settle_pinned(&mut self) {
        for (read, owner) in std::mem::take(&mut self.pinned) {
            if !self.waiting_reads.contains(&read) {
                (self.settled)
                    .entry(read)
                    .or_insert(SettledRead::Owner(owner));
            }
        }
    }
}
