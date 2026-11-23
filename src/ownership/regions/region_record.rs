use super::definition_index::DefinitionIndex;
use super::result_borrows::ResultBorrows;
use crate::modules::ModuleSources;
use crate::types::{Regions, ResultRegion, TypedExprKind, TypedProgram};

pub struct RegionRecord;

impl RegionRecord {
    pub fn of(program: &TypedProgram) -> Regions {
        let index = DefinitionIndex::of(program);
        let analysis = ResultBorrows::new(program, &index);
        let mut regions = Regions::default();
        let user_definitions = program
            .modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.definitions);
        for definition in user_definitions {
            let TypedExprKind::Fn(lambda) = &definition.value.kind else {
                continue;
            };
            let flows = analysis.analyse_function(definition.binding, lambda).flows;
            let region = ResultRegion::new(flows.flows, flows.higher_ranked);
            regions.record(definition.binding, region);
        }
        regions
    }
}
