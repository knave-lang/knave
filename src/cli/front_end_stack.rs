use crate::stack::FreshStack;

pub struct FrontEndStack;

impl FrontEndStack {
    // A debug build checks a program nested to `Form::NESTING_LIMIT` in about 7.4 MiB of stack.
    // This size gives four times that room, and reserves address space, not memory.
    const SIZE: usize = 32 * 1024 * 1024;
    const STACK: FreshStack = FreshStack::of(Self::SIZE);

    pub fn run<R>(work: impl FnOnce() -> R) -> R {
        Self::STACK.run(work)
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::panic;

    use super::FrontEndStack;

    struct StackHungryRecursion;

    impl StackHungryRecursion {
        const FRAME_BYTES: usize = 1024;
        const DEFAULT_MAIN_STACK: usize = 8 * 1024 * 1024;
        const LEVELS: usize = 3 * Self::DEFAULT_MAIN_STACK / (2 * Self::FRAME_BYTES);

        fn depth_reached(levels: usize) -> usize {
            let frame = black_box([0u8; Self::FRAME_BYTES]);
            match levels {
                0 => usize::from(frame[0]),
                _ => 1 + Self::depth_reached(black_box(levels - 1)),
            }
        }
    }

    #[test]
    fn work_deeper_than_the_default_main_stack_runs_on_the_front_end_stack() {
        let levels = StackHungryRecursion::LEVELS;
        let reached = FrontEndStack::run(|| StackHungryRecursion::depth_reached(levels));
        assert_eq!(reached, levels);
    }

    #[test]
    fn a_panic_on_the_front_end_stack_reaches_the_caller() {
        const MESSAGE: &str = "the front end failed";
        let payload = panic::catch_unwind(|| FrontEndStack::run(|| panic!("{MESSAGE}")))
            .expect_err("the panic reaches the caller");
        assert_eq!(
            payload.downcast_ref::<String>().map(String::as_str),
            Some(MESSAGE)
        );
    }
}
