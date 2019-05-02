
use std::fmt::{self, Formatter, Debug, Display};
use std::error::Error as StdError;

use corona::errors::TaskFailed as CoronaError;
use tokio::io::Error as TokioError;

pub enum JokerError {
    Corona {
        debug: String,
        debug_alternate: String,
        display: String,
    },
    Tokio(TokioError),
    Other(failure::Error),
}

impl Debug for JokerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            &JokerError::Corona {
                ref debug,
                ref debug_alternate,
                ..
            } => match f.alternate() {
                true => f.write_str(debug_alternate),
                false => f.write_str(debug),
            },
            &JokerError::Tokio(ref error) => f.debug_tuple("Tokio")
                .field(error)
                .finish(),
            &JokerError::Other(ref error) => f.debug_tuple("Other")
                .field(error)
                .finish(),
        }
    }
}

impl Display for JokerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            &JokerError::Corona {
                ref display,
                ..
            } => f.write_str(display),
            &JokerError::Tokio(ref error) => Display::fmt(error, f),
            &JokerError::Other(ref error) => Display::fmt(error, f),
        }
    }
}

impl StdError for JokerError {}

impl From<CoronaError> for JokerError {
    fn from(inner: CoronaError) -> Self {
        JokerError::Corona {
            debug: format!("{:?}", inner),
            debug_alternate: format!("{:#?}", inner),
            display: format!("{}", inner),
        }
    }
}

impl From<TokioError> for JokerError {
    fn from(inner: TokioError) -> Self {
        JokerError::Tokio(inner)
    }
}

impl From<failure::Error> for JokerError {
    fn from(inner: failure::Error) -> Self {
        JokerError::Other(inner)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub enum Infallible {}

impl Display for Infallible {
    fn fmt(&self, _: &mut Formatter) -> fmt::Result {
        match *self {}
    }
}

impl StdError for Infallible {}

impl From<Infallible> for JokerError {
    fn from(never: Infallible) -> Self {
        match never {}
    }
}