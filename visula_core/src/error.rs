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
}
