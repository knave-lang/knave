use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::helper_template::{HelperArgument, HelperTemplate};
use crate::production::Production;

pub struct CallHelper;

impl CallHelper {
    fn argument(state: &mut GeneratorState, argument: HelperArgument) -> Option<String> {
        match argument {
            HelperArgument::Local(kind) => state.some(kind).map(str::to_string),
            HelperArgument::Borrowed(kind) => state.some(kind).map(|name| format!("(ref {name})")),
            HelperArgument::Function(template) => Some(template.name.to_string()),
            HelperArgument::Expression => Some(Expressions::number(state)),
        }
    }
}

impl Production for CallHelper {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let template: &'static HelperTemplate = state.pick(&HelperTemplate::ALL)?;
        let arguments = template
            .arguments()
            .iter()
            .map(|argument| Self::argument(state, *argument))
            .collect::<Option<Vec<String>>>()?;
        state.use_helper(template);
        for argument in template.arguments() {
            if let HelperArgument::Function(passed) = argument {
                state.use_helper(passed);
            }
        }
        let call = format!("({} {})", template.name, arguments.join(" "));
        Some(match template.result() {
            Some(kind) => state.bind_local(kind, call),
            None => call,
        })
    }
}
