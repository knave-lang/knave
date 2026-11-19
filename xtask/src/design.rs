mod defining_modules;
mod module_address;
mod module_graph;
mod module_names;
mod signatures;
mod type_shapes;
mod use_bindings;
mod use_paths;

use std::path::Path;

use module_address::Stage;
use module_graph::ModuleGraph;
use signatures::Signatures;
use type_shapes::TypeShapes;

use crate::census::Census;
use crate::failure::TaskFailure;
use crate::parsed_source::ParsedSource;
use crate::production_sources::ProductionSources;
use crate::ratchet::{Limit, Measurement, Unit};

pub struct DesignRules;

impl DesignRules {
    const MAXIMUM_METHODS_PER_TYPE: usize = 20;
    const MAXIMUM_NON_PRIVATE_METHODS_PER_TYPE: usize = 7;
    const MAXIMUM_FIELDS_PER_STRUCT: usize = 10;
    const MAXIMUM_FILES_WITH_INHERENT_IMPLS_PER_TYPE: usize = 1;
    const MAXIMUM_ARGUMENTS_PER_FUNCTION: usize = 3;
    const MAXIMUM_BOOL_PARAMETERS_PER_FUNCTION: usize = 0;
    const MAXIMUM_FREE_FUNCTIONS: usize = 0;
    const MAXIMUM_CYCLE_LINKS_PER_MODULE: usize = 0;
    const MAXIMUM_LATER_STAGE_DEPENDENCIES_PER_MODULE: usize = 0;
    const STAGE_ORDER: [Stage; 8] = [
        Stage::new("expand/src", "reader"),
        Stage::new("expand/src", "expand"),
        Stage::new("src", "types"),
        Stage::new("src", "ownership"),
        Stage::new("src", "lint"),
        Stage::new("src", "emit"),
        Stage::new("src", "build"),
        Stage::new("src", "cli"),
    ];
    const EXTRACT_AN_OBJECT: &str = "Extract an object.";

    const METHODS: Limit = Limit {
        name: "methods_per_type",
        maximum: Self::MAXIMUM_METHODS_PER_TYPE,
        subjects: "types",
        unit: Unit::new("method", "methods"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const NON_PRIVATE_METHODS: Limit = Limit {
        name: "non_private_methods_per_type",
        maximum: Self::MAXIMUM_NON_PRIVATE_METHODS_PER_TYPE,
        subjects: "types",
        unit: Unit::new("non-private method", "non-private methods"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const FIELDS: Limit = Limit {
        name: "fields_per_struct",
        maximum: Self::MAXIMUM_FIELDS_PER_STRUCT,
        subjects: "structs",
        unit: Unit::new("field", "fields"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const INHERENT_IMPL_FILES: Limit = Limit {
        name: "files_with_inherent_impls_per_type",
        maximum: Self::MAXIMUM_FILES_WITH_INHERENT_IMPLS_PER_TYPE,
        subjects: "types",
        unit: Unit::new("file with inherent impls", "files with inherent impls"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const ARGUMENTS: Limit = Limit {
        name: "arguments_per_function",
        maximum: Self::MAXIMUM_ARGUMENTS_PER_FUNCTION,
        subjects: "functions",
        unit: Unit::new("argument besides self", "arguments besides self"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const BOOL_PARAMETERS: Limit = Limit {
        name: "bool_parameters_per_function",
        maximum: Self::MAXIMUM_BOOL_PARAMETERS_PER_FUNCTION,
        subjects: "functions",
        unit: Unit::new("bool parameter", "bool parameters"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const FREE_FUNCTIONS: Limit = Limit {
        name: "free_functions",
        maximum: Self::MAXIMUM_FREE_FUNCTIONS,
        subjects: "functions",
        unit: Unit::new("free function definition", "free function definitions"),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const CYCLE_LINKS: Limit = Limit {
        name: "cycle_links_per_module",
        maximum: Self::MAXIMUM_CYCLE_LINKS_PER_MODULE,
        subjects: "modules",
        unit: Unit::new(
            "link to another module of its cycle",
            "links to other modules of its cycle",
        ),
        advice: Self::EXTRACT_AN_OBJECT,
    };
    const LATER_STAGE_DEPENDENCIES: Limit = Limit {
        name: "later_stage_dependencies",
        maximum: Self::MAXIMUM_LATER_STAGE_DEPENDENCIES_PER_MODULE,
        subjects: "stage modules",
        unit: Unit::new(
            "dependency on a later stage",
            "dependencies on a later stage",
        ),
        advice: Self::EXTRACT_AN_OBJECT,
    };

    fn measured(sources: &[ParsedSource]) -> [Measurement; 9] {
        let [methods, non_private_methods, fields, inherent_impl_files] =
            TypeShapes::measure(sources).into_counts();
        let [arguments, bool_parameters, free_functions] =
            Signatures::measure(sources).into_counts();
        let graph = ModuleGraph::build(sources);
        [
            Measurement {
                limit: Self::METHODS,
                counts: methods,
            },
            Measurement {
                limit: Self::NON_PRIVATE_METHODS,
                counts: non_private_methods,
            },
            Measurement {
                limit: Self::FIELDS,
                counts: fields,
            },
            Measurement {
                limit: Self::INHERENT_IMPL_FILES,
                counts: inherent_impl_files,
            },
            Measurement {
                limit: Self::ARGUMENTS,
                counts: arguments,
            },
            Measurement {
                limit: Self::BOOL_PARAMETERS,
                counts: bool_parameters,
            },
            Measurement {
                limit: Self::FREE_FUNCTIONS,
                counts: free_functions,
            },
            Measurement {
                limit: Self::CYCLE_LINKS,
                counts: graph.cycle_links(),
            },
            Measurement {
                limit: Self::LATER_STAGE_DEPENDENCIES,
                counts: graph.later_stage_dependencies(&Self::STAGE_ORDER),
            },
        ]
    }
}

impl Census for DesignRules {
    fn measure(&self, root: &Path) -> Result<Vec<Measurement>, TaskFailure> {
        let sources = ProductionSources::new(root).parsed()?;
        Ok(Self::measured(&sources).into())
    }
}
