//! # webpage-info
//!
//! A modern Rust library to extract metadata from web pages: title, description,
//! OpenGraph, Schema.org, links, and more.
//!
//! ## Features
//!
//! - Parse HTML from strings, files, or URLs
//! - Extract common metadata (title, description, language)
//! - Parse OpenGraph protocol data
//! - Parse Schema.org JSON-LD structured data
//! - Extract all links from the document
//! - Async HTTP client with configurable options
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use webpage_info::WebpageInfo;
//!
//! #[tokio::main]
//! async fn main() -> webpage_info::Result<()> {
//!     // Fetch and parse a webpage
//!     let info = WebpageInfo::fetch("https://example.org").await?;
//!
//!     println!("Title: {:?}", info.html.title);
//!     println!("Description: {:?}", info.html.description);
//!     println!("Links: {}", info.html.links.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Parsing Local HTML
//!
//! ```rust
//! use webpage_info::HtmlInfo;
//!
//! let html = "<html><head><title>Hello</title></head><body>World</body></html>";
//! let info = HtmlInfo::from_string(html, None).unwrap();
//! assert_eq!(info.title, Some("Hello".to_string()));
//! ```
//!
//! ## Custom HTTP Options
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use webpage_info::{WebpageInfo, HttpOptions};
//!
//! #[tokio::main]
//! async fn main() -> webpage_info::Result<()> {
//!     let options = HttpOptions::new()
//!         .timeout(Duration::from_secs(60))
//!         .user_agent("MyBot/1.0")
//!         .allow_insecure(true);
//!
//!     let info = WebpageInfo::fetch_with_options("https://example.org", options).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Without HTTP (parsing only)
//!
//! If you don't need HTTP fetching, disable the default `http` feature:
//!
//! ```toml
//! [dependencies]
//! webpage-info = { version = "1.0", default-features = false }
//! ```

mod error;
mod html;
mod opengraph;
mod schema_org;

#[cfg(feature = "http")]
mod http;

pub use error::{Error, Result};
pub use html::{HtmlInfo, Link};
pub use opengraph::{Opengraph, OpengraphMedia};
pub use schema_org::SchemaOrg;

#[cfg(feature = "http")]
pub use http::{HttpInfo, HttpOptions};

use serde::{Deserialize, Serialize};

/// Complete webpage information including HTTP and HTML data.
#[cfg(feature = "http")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpageInfo {
    /// HTTP transfer information
    pub http: HttpInfo,

    /// Parsed HTML information
    pub html: HtmlInfo,
}

#[cfg(feature = "http")]
impl WebpageInfo {
    /// Fetch a webpage from a URL with default options.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use webpage_info::WebpageInfo;
    ///
    /// #[tokio::main]
    /// async fn main() -> webpage_info::Result<()> {
    ///     let info = WebpageInfo::fetch("https://example.org").await?;
    ///     println!("Title: {:?}", info.html.title);
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch(url: &str) -> Result<Self> {
        Self::fetch_with_options(url, HttpOptions::default()).await
    }

    /// Fetch a webpage from a URL with custom HTTP options.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    /// use webpage_info::{WebpageInfo, HttpOptions};
    ///
    /// #[tokio::main]
    /// async fn main() -> webpage_info::Result<()> {
    ///     let options = HttpOptions::new()
    ///         .timeout(Duration::from_secs(60))
    ///         .user_agent("CustomBot/1.0");
    ///
    ///     let info = WebpageInfo::fetch_with_options("https://example.org", options).await?;
    ///     println!("Status: {}", info.http.status_code);
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_with_options(url: &str, options: HttpOptions) -> Result<Self> {
        let http_info = http::fetch(url, &options).await?;

        // Validate content type is HTML-ish
        if let Some(ref ct) = http_info.content_type
            && !ct.contains("html")
            && !ct.contains("xml")
        {
            return Err(Error::InvalidContentType(ct.clone()));
        }

        let html = HtmlInfo::from_string(&http_info.body, Some(&http_info.url))?;

        Ok(Self {
            http: http_info,
            html,
        })
    }
}
