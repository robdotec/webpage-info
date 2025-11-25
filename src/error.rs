//! Error types for webpage-info

use thiserror::Error;

/// Errors that can occur when fetching or parsing webpage information.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to parse or validate the URL
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    /// URL parse error (from url crate)
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// HTTP request failed
    #[cfg(feature = "http")]
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to read file
    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),

    /// HTML parsing error
    #[error("failed to parse HTML")]
    ParseError,

    /// Invalid response (non-HTML content type)
    #[error("invalid content type: expected HTML, got {0}")]
    InvalidContentType(String),

    /// Request blocked due to SSRF protection
    #[cfg(feature = "http")]
    #[error("SSRF protection: {0}")]
    SsrfBlocked(String),
}

/// Result type alias for webpage-info operations.
pub type Result<T> = std::result::Result<T, Error>;
