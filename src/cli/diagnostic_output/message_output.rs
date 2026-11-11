use crate::cli::standard_error::StandardError;

pub trait MessageOutput {
    fn write_error(&self, message: &str) {
        StandardError::write_line(format_args!("error: {message}"));
    }

    fn write_warning(&self, message: &str) {
        StandardError::write_line(format_args!("warning: {message}"));
    }
}
