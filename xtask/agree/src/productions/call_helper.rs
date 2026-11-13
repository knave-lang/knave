use crate::block_writer::BlockWriter;
use crate::expressions::Expressions;
use crate::generator_state::GeneratorState;
use crate::helper_template::{HelperArgument, HelperTemplate};
use crate::production::Production;
use crate::shape::Shape;

pub struct CallHelper;

impl CallHelper {
    fn callable(state: &GeneratorState) -> Vec<&'static HelperTemplate> {
        HelperTemplate::ALL
            .into_iter()
            .filter(|template| {
                template
                    .arguments()
                    .iter()
                    .all(|argument| Self::can_pass(state, *argument))
            })
            .collect()
    }

    fn can_pass(state: &GeneratorState, argument: HelperArgument) -> bool {
        match argument {
            HelperArgument::Local(kind) | HelperArgument::Borrowed(kind) => state.holds(kind),
            HelperArgument::Function(_) | HelperArgument::Expression => true,
        }
    }

    fn argument(state: &mut GeneratorState, argument: HelperArgument) -> Option<String> {
        match argument {
            HelperArgument::Local(kind) => state.some(kind).map(str::to_string),
            HelperArgument::Borrowed(kind) => {
                state.some(kind).map(|name| Shape::BORROWED.filled(name))
            }
            HelperArgument::Function(template) => Some(template.name.to_string()),
            HelperArgument::Expression => Some(Expressions::number(state)),
        }
    }
}

impl Production for CallHelper {
    fn write(&self, _blocks: &dyn BlockWriter, state: &mut GeneratorState) -> Option<String> {
        let callable = Self::callable(state);
        let template = state.pick(&callable)?;
        let arguments = template
            .arguments()
            .iter()
            .map(|argument| Self::argument(state, *argument))
            .collect::<Option<Vec<String>>>()?;
        state.use_helper(template);
        let call = format!("({} {})", template.name, arguments.join(" "));
        Some(match template.result() {
            Some(kind) => state.bind_local(kind, call),
            None => call,
        })
    }
}
