mod module_graph;
mod signatures;
mod type_shapes;

use std::path::Path;

use module_graph::{ModuleGraph, Stage};
use signatures::Signatures;
use type_shapes::TypeShapes;

use crate::failure::TaskFailure;
use crate::parsed_source::ParsedSource;
use crate::ratchet::{Counts, Limit, Ratchet, Unit};

pub struct DesignRules;

impl DesignRules {
    const MAXIMUM_METHODS_PER_TYPE: usize = 20;
    const MAXIMUM_NON_PRIVATE_METHODS_PER_TYPE: usize = 7;
    const MAXIMUM_FIELDS_PER_STRUCT: usize = 10;
    const MAXIMUM_FILES_WITH_INHERENT_IMPLS_PER_TYPE: usize = 1;
    const MAXIMUM_ARGUMENTS_PER_FUNCTION: usize = 3;
    const MAXIMUM_BOOL_PARAMETERS_PER_FUNCTION: usize = 0;
    const MAXIMUM_FREE_FUNCTIONS: usize = 0;
    const MAXIMUM_CYCLE_PARTNERS_PER_MODULE: usize = 0;
    const MAXIMUM_LATER_STAGE_DEPENDENCIES_PER_MODULE: usize = 0;
    const STAGE_ORDER: [Stage; 7] = [
        ("expand/src", "reader"),
        ("expand/src", "expand"),
        ("src", "types"),
        ("src", "ownership"),
        ("src", "emit"),
        ("src", "build"),
        ("src", "cli"),
    ];
    const EXTRACT_AN_OBJECT: &str = "Extract an object.";

    const METHODS: Limit = Limit {
        baseline: "xtask/baselines/methods_per_type.txt",
        maximum: Self::MAXIMUM_METHODS_PER_TYPE,
        subjects: "types",
        unit: Unit::new("method", "methods"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const NON_PRIVATE_METHODS: Limit = Limit {
        baseline: "xtask/baselines/non_private_methods_per_type.txt",
        maximum: Self::MAXIMUM_NON_PRIVATE_METHODS_PER_TYPE,
        subjects: "types",
        unit: Unit::new("non-private method", "non-private methods"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const FIELDS: Limit = Limit {
        baseline: "xtask/baselines/fields_per_struct.txt",
        maximum: Self::MAXIMUM_FIELDS_PER_STRUCT,
        subjects: "structs",
        unit: Unit::new("field", "fields"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const INHERENT_IMPL_FILES: Limit = Limit {
        baseline: "xtask/baselines/files_with_inherent_impls_per_type.txt",
        maximum: Self::MAXIMUM_FILES_WITH_INHERENT_IMPLS_PER_TYPE,
        subjects: "types",
        unit: Unit::new("file with inherent impls", "files with inherent impls"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const ARGUMENTS: Limit = Limit {
        baseline: "xtask/baselines/arguments_per_function.txt",
        maximum: Self::MAXIMUM_ARGUMENTS_PER_FUNCTION,
        subjects: "functions",
        unit: Unit::new("argument besides self", "arguments besides self"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const BOOL_PARAMETERS: Limit = Limit {
        baseline: "xtask/baselines/bool_parameters_per_function.txt",
        maximum: Self::MAXIMUM_BOOL_PARAMETERS_PER_FUNCTION,
        subjects: "functions",
        unit: Unit::new("bool parameter", "bool parameters"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const FREE_FUNCTIONS: Limit = Limit {
        baseline: "xtask/baselines/free_functions.txt",
        maximum: Self::MAXIMUM_FREE_FUNCTIONS,
        subjects: "functions",
        unit: Unit::new("free function definition", "free function definitions"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const CYCLE_PARTNERS: Limit = Limit {
        baseline: "xtask/baselines/module_cycle_partners.txt",
        maximum: Self::MAXIMUM_CYCLE_PARTNERS_PER_MODULE,
        subjects: "modules",
        unit: Unit::new("module in a cycle with it", "modules in a cycle with it"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const LATER_STAGE_DEPENDENCIES: Limit = Limit {
        baseline: "xtask/baselines/later_stage_dependencies.txt",
        maximum: Self::MAXIMUM_LATER_STAGE_DEPENDENCIES_PER_MODULE,
        subjects: "stage modules",
        unit: Unit::new(
            "dependency on a later stage",
            "dependencies on a later stage",
        ),
        advice: Self::EXTRACT_AN_OBJECT,
    };

    pub fn ratchets(root: &Path, sources: &[ParsedSource]) -> Result<Vec<Ratchet>, TaskFailure> {
        Self::measured(sources)
            .into_iter()
            .map(|(limit, measured)| Ratchet::read(root, limit, measured))
            .collect()
    }

    fn measured(sources: &[ParsedSource]) -> [(Limit, Counts); 9] {
        let [methods, non_private_methods, fields, inherent_impl_files] =
            TypeShapes::measure(sources).into_counts();
        let [arguments, bool_parameters, free_functions] =
            Signatures::measure(sources).into_counts();
        let graph = ModuleGraph::build(sources);
        [
            (Self::METHODS, methods),
            (Self::NON_PRIVATE_METHODS, non_private_methods),
            (Self::FIELDS, fields),
            (Self::INHERENT_IMPL_FILES, inherent_impl_files),
            (Self::ARGUMENTS, arguments),
            (Self::BOOL_PARAMETERS, bool_parameters),
            (Self::FREE_FUNCTIONS, free_functions),
            (Self::CYCLE_PARTNERS, graph.cycle_partners()),
            (
                Self::LATER_STAGE_DEPENDENCIES,
                graph.later_stage_dependencies(&Self::STAGE_ORDER),
            ),
        ]
    }
}
