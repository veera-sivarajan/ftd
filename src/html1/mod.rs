#[cfg(test)]
#[macro_use]
mod test;

mod data;
mod dependencies;
mod events;
mod functions;
mod main;
pub mod utils;
mod variable_dependencies;

pub use events::Action;
pub use functions::{ExpressionGenerator, FunctionGenerator};
pub use main::HtmlUI;
pub use variable_dependencies::VariableDependencyGenerator;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("InterpreterError: {}", _0)]
    InterpreterError(#[from] ftd::interpreter2::Error),

    #[error("{doc_id}:{line_number} -> {message}")]
    ParseError {
        message: String,
        doc_id: String,
        line_number: usize,
    },

    #[error("InterpretEvalexprErrorerError: {}", _0)]
    EvalexprError(#[from] ftd::evalexpr::EvalexprError),
}

pub type Result<T> = std::result::Result<T, Error>;
