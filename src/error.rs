use winnow::error::{ContextError, ErrMode};

type WinnowError = ErrMode<ContextError>;

#[derive(Debug)]
pub enum ConfigError {
    MissingRequiredField{ section: String, field: &'static str },
    FieldParseError { section: String, field: &'static str, msg: String },
    Other(String),
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Error {
    ReadError(std::io::Error),
    ParseError(WinnowError),
    ConfigError(ConfigError),
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
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::ReadError(value)
    }
}
impl From<WinnowError> for Error {
    fn from(value: WinnowError) -> Self {
        Self::ParseError(value)
    }
}
