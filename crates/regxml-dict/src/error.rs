use thiserror::Error;

#[derive(Debug, Error)]
pub enum DictError {
    #[error("XML parse error: {0}")]
    Xml(String),
    #[error("duplicate definition: {0}")]
    DuplicateDefinition(String),
    #[error("missing required field '{0}' in entry '{1}'")]
    MissingField(&'static str, String),
    #[error("unknown TypeKind '{0}'")]
    UnknownTypeKind(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
