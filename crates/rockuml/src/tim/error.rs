use crate::json::ParseError;
use crate::text::StringLocated;

/// A preprocessing failure reported to the user at a source line.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct EaterException {
    pub message: String,
    pub location: StringLocated,
}

impl EaterException {
    pub(super) fn new(message: impl Into<String>, location: &StringLocated) -> Self {
        Self {
            message: message.into(),
            location: location.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum TimError {
    Eater(EaterException),
    /// Raised by JSON literals; multi-line JSON assignments catch it to read the next line and retry.
    JsonParse(ParseError),
    /// Any other Java runtime exception: PlantUML reports these as "Fatal parsing error".
    Fatal,
}

impl From<EaterException> for TimError {
    fn from(error: EaterException) -> Self {
        TimError::Eater(error)
    }
}

impl From<ParseError> for TimError {
    fn from(error: ParseError) -> Self {
        TimError::JsonParse(error)
    }
}

pub(super) type TimResult<T> = Result<T, TimError>;

pub(super) fn fail<T>(message: impl Into<String>, location: &StringLocated) -> TimResult<T> {
    Err(TimError::Eater(EaterException::new(message, location)))
}
