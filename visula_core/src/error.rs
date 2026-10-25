use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("failed to parse shader: {0}")]
    Parse(#[from] naga::front::wgsl::ParseError),
    #[error("shader validation failed: {0}")]
    Validation(#[from] Box<naga::WithSpan<naga::valid::ValidationError>>),
    #[error("failed to write shader: {0}")]
    Write(#[from] naga::back::wgsl::Error),
    #[error("entry point '{0}' not found in shader")]
    EntryPointNotFound(String),
    #[error("variable '{0}' not found in shader")]
    VariableNotFound(String),
    #[error("function '{0}' not found in shader")]
    FunctionNotFound(String),
    #[error("failed to resolve the type of an expression: {0}")]
    Resolve(#[from] naga::proc::ResolveError),
    #[error("unsupported operand types: {left} {operator} {right}")]
    UnsupportedOperandTypes {
        operator: &'static str,
        left: String,
        right: String,
    },
    #[error("unsupported operand type: {operator}{operand}")]
    UnsupportedOperandType {
        operator: &'static str,
        operand: String,
    },
    #[error("unsupported argument types: {function}({arguments})")]
    UnsupportedArgumentTypes { function: String, arguments: String },
}
