use winnow::error::{ContextError, ErrMode};

type WinnowError = ErrMode<ContextError>;

#[derive(Debug)]
pub enum Error {
    ReadError(std::io::Error),
    ParseError(WinnowError),
    ConfigError(String),
}
impl Error {
    pub fn config_error<T>(x: T) -> Self
    where
        T: Into<String>,
    {
        Self::ConfigError(x.into())
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
