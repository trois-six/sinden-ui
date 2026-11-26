use std::io;

/// An error talking to the driver service.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The endpoint could not be reached or the connection failed.
    #[error("connection to the driver failed: {0}")]
    Io(#[from] io::Error),
    /// The driver closed the connection.
    #[error("the driver closed the connection")]
    Closed,
    /// A line was not valid JSON or did not have the expected shape.
    #[error("invalid message from the driver: {0}")]
    Json(#[from] serde_json::Error),
    /// The driver answered something the protocol does not allow.
    #[error("protocol error: {0}")]
    Protocol(String),
    /// The driver answered the request with an error.
    #[error("the driver refused the request: {0}")]
    Service(String),
}

impl Error {
    /// Whether the error means the driver is not reachable (not running,
    /// no permission on the endpoint, connection lost), as opposed to a
    /// request it refused.
    #[must_use]
    pub fn is_unreachable(&self) -> bool {
        matches!(self, Self::Io(_) | Self::Closed)
    }
}
