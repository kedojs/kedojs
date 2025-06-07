use std::{convert::Infallible, error::Error, fmt::Debug, io};

pub(crate) type BoxError = Box<dyn Error + Send + Sync>;

pub struct FetchError {
    pub message: String,
    pub inner: Option<BoxError>,
}

impl FetchError {
    pub fn new(message: &str) -> Self {
        FetchError {
            message: message.to_string(),
            inner: None,
        }
    }

    pub fn with_error<E: Error + Send + Sync + 'static>(message: &str, error: E) -> Self {
        FetchError {
            message: message.to_string(),
            inner: Some(Box::new(error)),
        }
    }
}

impl From<hyper::Error> for FetchError {
    fn from(error: hyper::Error) -> Self {
        FetchError {
            message: error.to_string(),
            inner: Some(Box::new(error)),
        }
    }
}

impl From<hyper_util::client::legacy::Error> for FetchError {
    fn from(error: hyper_util::client::legacy::Error) -> Self {
        FetchError {
            message: error.to_string(),
            inner: Some(Box::new(error)),
        }
    }
}

impl From<FetchError> for io::Error {
    fn from(error: FetchError) -> Self {
        io::Error::new(io::ErrorKind::Other, error.message)
    }
}

impl From<io::Error> for FetchError {
    fn from(error: io::Error) -> Self {
        FetchError {
            message: error.to_string(),
            inner: Some(Box::new(error)),
        }
    }
}

impl From<Infallible> for FetchError {
    fn from(_: Infallible) -> Self {
        FetchError {
            message: "Infallible error".to_string(),
            inner: None,
        }
    }
}

impl From<hyper::header::ToStrError> for FetchError {
    fn from(error: hyper::header::ToStrError) -> Self {
        FetchError {
            message: error.to_string(),
            inner: Some(Box::new(error)),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for FetchError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        FetchError {
            message: error.to_string(),
            inner: Some(error),
        }
    }
}

impl Error for FetchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Debug for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl FetchError {
    pub fn describe(&self) -> String {
        let mut message = self.message.clone();
        if let Some(inner) = self.inner.as_ref().and_then(|e| e.source()) {
            message.push_str(&format!("\n{}", inner));
        }
        message
    }
}
