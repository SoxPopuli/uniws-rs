use winnow::error::{ContextError, ErrMode};

type WinnowError = ErrMode<ContextError>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConfigError {
    MissingRequiredField{ section: String, field: &'static str },
    FieldParseError { section: String, field: &'static str, msg: String },
    Other(String),
}

#[allow(clippy::enum_variant_names)]
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Error {
    ReadError(std::io::ErrorKind),
    ParseError(WinnowError),
    ConfigError(ConfigError),
    StateError(String),
    PatchError { iteration: usize },
}
impl Error {
    pub fn config_missing_field(section: impl Into<String>, field: &'static str) -> Self {
        Self::ConfigError( ConfigError::MissingRequiredField{ section: section.into(), field })
    }

    pub fn config_error<T>(x: T) -> Self
    where
        T: Into<String>,
    {
        Self::ConfigError(ConfigError::Other(x.into()))
    }

    pub fn config_field_parse(section: impl Into<String>, field: &'static str, msg: impl Into<String>) -> Self
    {
        Self::ConfigError(ConfigError::FieldParseError {
            section: section.into(),
            field,
            msg: msg.into(),
        })
    }

    pub fn state_error(msg: impl Into<String>) -> Self {
        Self::StateError(msg.into())
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        let kind = value.kind();
        Self::ReadError(kind)
    }
}
impl From<WinnowError> for Error {
    fn from(value: WinnowError) -> Self {
        Self::ParseError(value)
    }
}
