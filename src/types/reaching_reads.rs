use super::copy::CopyRules;
use super::display::TypeDisplay;
use super::field_reader::FieldReader;
use super::open_read::OpenRead;
use super::pinning::InstantiationReport;
use super::reaching_read::ReachingRead;
use super::reaching_structs::ReachingStructs;
use super::ty::Type;
use super::typed::TypedProgram;
use super::written_refusal::{ReadRefusal, WrittenRefusal};

pub(super) struct ReachingReads<'r> {
    report: &'r InstantiationReport,
    program: &'r TypedProgram,
}

impl<'r> ReachingReads<'r> {
    pub(super) fn new(report: &'r InstantiationReport, program: &'r TypedProgram) -> Self {
        Self { report, program }
    }

    pub(super) fn shown(typed: &TypedProgram, reads: &[ReachingRead]) -> Vec<(String, String)> {
        let display = TypeDisplay::new(&typed.bindings, &typed.generics);
        let mut shown: Vec<(String, String)> = Vec::new();
        for reached in reads {
            let owner = Type::Named(reached.owner, reached.arguments.clone());
            let entry = (display.show(&owner), display.show(&reached.read.read));
            if !shown.contains(&entry) {
                shown.push(entry);
            }
        }
        shown
    }

    pub(super) fn of(&self, reader: &FieldReader) -> Result<Vec<ReachingRead>, WrittenRefusal> {
        let open_struct = WrittenRefusal::Read(ReadRefusal::OpenStruct);
        let structs = ReachingStructs::new(self.report)
            .of(reader.binding, reader.owner)
            .ok_or(open_struct)?;
        let declarations = &self.program.declarations;
        let reads = OpenRead::new(CopyRules::new(declarations, &self.program.generics));
        let mut found = Vec::new();
        for (owner, arguments) in structs {
            if !arguments.iter().all(Type::is_concrete) {
                return Err(open_struct);
            }
            let definition = declarations.types().structs.get(owner);
            let read =
                definition.and_then(|known| reads.through_bound(known, &reader.field, &arguments));
            if let Some(read) = read {
                found.push(ReachingRead {
                    owner,
                    arguments,
                    read,
                });
            }
        }
        Ok(found)
    }
}
