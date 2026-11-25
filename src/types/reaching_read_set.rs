use super::display::TypeDisplay;
use super::reaching_read::ReachingRead;
use super::ty::Type;
use super::typed::TypedProgram;

pub(super) struct ReachingReadSet(Vec<ReachingRead>);

impl ReachingReadSet {
    pub(super) fn new(reads: Vec<ReachingRead>) -> Self {
        Self(reads)
    }

    pub(super) fn distinct(&self) -> Vec<&ReachingRead> {
        let mut found: Vec<&ReachingRead> = Vec::new();
        for reached in &self.0 {
            if !found.iter().any(|known| known.read == reached.read) {
                found.push(reached);
            }
        }
        found
    }

    pub(super) fn differ(&self) -> bool {
        self.distinct().len() > 1
    }

    pub(super) fn reads_function(&self) -> bool {
        (self.0.iter()).any(|reached| matches!(reached.read.read, Type::Function(..)))
    }

    pub(super) fn shown(&self, program: &TypedProgram) -> Vec<(String, String)> {
        let display = TypeDisplay::new(&program.bindings, &program.generics);
        let mut by_owner: Vec<&ReachingRead> = self.0.iter().collect();
        by_owner.sort_by_key(|reached| reached.owner);
        let mut shown: Vec<(String, String)> = Vec::new();
        for reached in by_owner {
            let owner = Type::Named(reached.owner, reached.arguments.clone());
            let entry = (display.show(&owner), display.show(&reached.read.read));
            if !shown.contains(&entry) {
                shown.push(entry);
            }
        }
        shown
    }

    pub(super) fn into_reads(self) -> Vec<ReachingRead> {
        self.0
    }
}
