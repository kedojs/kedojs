use std::io::ErrorKind;

#[derive(Debug)]
pub enum KedoError {
    IoError(std::io::Error),
    HyperError(hyper::Error),
    Basic(ErrorKind, String),
    JsError(rust_jsc::JSError),
}

#[allow(dead_code)]
pub type KedoResult<T> = Result<T, KedoError>;

impl KedoError {
    #[allow(dead_code)]
    pub fn new(kind: ErrorKind, message: &str) -> KedoError {
        KedoError::Basic(kind, message.to_string())
    }
}

impl std::fmt::Display for KedoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            KedoError::IoError(err) => write!(f, "IO Error: {}", err),
            KedoError::HyperError(err) => write!(f, "Hyper Error: {}", err),
            KedoError::JsError(err) => {
                write!(f, "JS Error: {}", err.message().unwrap_or("unknown".into()))
            }
            KedoError::Basic(kind, message) => {
                write!(f, "Error: {:?} - {}", kind, message)
            }
        }
    }
}

impl From<std::io::Error> for KedoError {
    fn from(error: std::io::Error) -> Self {
        KedoError::IoError(error)
    }
}

impl From<hyper::Error> for KedoError {
    fn from(error: hyper::Error) -> Self {
        KedoError::HyperError(error)
    }
}

impl From<rust_jsc::JSError> for KedoError {
    fn from(error: rust_jsc::JSError) -> Self {
        KedoError::JsError(error)
    }
}

impl From<(ErrorKind, String)> for KedoError {
    fn from((kind, message): (ErrorKind, String)) -> Self {
        KedoError::Basic(kind, message)
    }
}

impl From<(ErrorKind, &str)> for KedoError {
    fn from((kind, message): (ErrorKind, &str)) -> Self {
        KedoError::Basic(kind, message.to_string())
    }
}

impl std::error::Error for KedoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KedoError::IoError(err) => Some(err),
            KedoError::HyperError(err) => Some(err),
            KedoError::JsError(err) => Some(err),
            KedoError::Basic(_, _) => None,
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            KedoError::IoError(err) => Some(err),
            KedoError::HyperError(err) => Some(err),
            KedoError::JsError(err) => Some(err),
            KedoError::Basic(_, _) => None,
        }
    }
}
