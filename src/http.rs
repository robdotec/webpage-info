//! HTTP client for fetching web pages

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Error, Result};

const DEFAULT_MAX_REDIRECTS: usize = 10;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// HTTP response information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInfo {
    /// The final URL after following redirects
    pub url: String,

    /// HTTP status code
    pub status_code: u16,

    /// Response headers
    pub headers: Vec<(String, String)>,

    /// Content-Type header value
    pub content_type: Option<String>,

    /// Number of redirects followed.
    ///
    /// Note: This is currently always 0 as reqwest doesn't expose redirect count directly.
    /// The field is retained for API compatibility and potential future implementation.
    pub redirect_count: u32,

    /// Response body as string
    pub body: String,
}

/// Configuration for HTTP requests.
#[derive(Debug, Clone)]
pub struct HttpOptions {
    /// Allow insecure HTTPS connections (self-signed certs).
    ///
    /// **Security Warning:** Enabling this allows man-in-the-middle attacks.
    /// Only use for testing or when connecting to known self-signed services.
    pub allow_insecure: bool,

    /// Follow HTTP redirects
    pub follow_redirects: bool,

    /// Maximum number of redirects to follow
    pub max_redirects: usize,

    /// Request timeout
    pub timeout: Duration,

    /// Maximum response body size in bytes.
    ///
    /// Responses larger than this will be truncated to prevent memory exhaustion.
    /// Default: 10 MB.
    pub max_body_size: usize,

    /// Block requests to private/internal IP addresses (SSRF protection).
    ///
    /// When enabled, requests to localhost, private networks (10.x, 172.16-31.x, 192.168.x),
    /// link-local addresses, and cloud metadata endpoints (169.254.x) are blocked.
    /// Default: true.
    pub block_private_ips: bool,

    /// User-Agent header
    pub user_agent: String,

    /// Additional headers to send
    pub headers: Vec<(String, String)>,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            allow_insecure: false,
            follow_redirects: true,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            block_private_ips: true,
            user_agent: format!(
                "webpage-info/{} (https://crates.io/crates/webpage-info)",
                env!("CARGO_PKG_VERSION")
            ),
            headers: Vec::new(),
        }
    }
}

impl HttpOptions {
    /// Create a new HttpOptions with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to allow insecure HTTPS connections.
    pub fn allow_insecure(mut self, allow: bool) -> Self {
        self.allow_insecure = allow;
        self
    }

    /// Set whether to follow redirects.
    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.follow_redirects = follow;
        self
    }

    /// Set the maximum number of redirects to follow.
    pub fn max_redirects(mut self, max: usize) -> Self {
        self.max_redirects = max;
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum response body size in bytes.
    ///
    /// Responses larger than this will be truncated.
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Set whether to block requests to private/internal IP addresses.
    ///
    /// **Security Note:** Disabling this exposes your application to SSRF attacks
    /// if URLs come from untrusted sources.
    pub fn block_private_ips(mut self, block: bool) -> Self {
        self.block_private_ips = block;
        self
    }

    /// Set the User-Agent header.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Build a reqwest Client from these options.
    fn build_client(&self) -> Result<Client> {
        let redirect_policy = if self.follow_redirects {
            reqwest::redirect::Policy::limited(self.max_redirects)
        } else {
            reqwest::redirect::Policy::none()
        };

        let mut builder = Client::builder()
            .danger_accept_invalid_certs(self.allow_insecure)
            .redirect(redirect_policy)
            .timeout(self.timeout)
            .user_agent(&self.user_agent);

        // Add default headers
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in &self.headers {
            if let (Ok(name), Ok(value)) = (
                name.parse::<reqwest::header::HeaderName>(),
                value.parse::<reqwest::header::HeaderValue>(),
            ) {
                headers.insert(name, value);
            }
        }
        builder = builder.default_headers(headers);

        Ok(builder.build()?)
    }
}

/// Check if an IPv4 address is private/internal.
fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()                           // 127.0.0.0/8
        || ip.is_private()                     // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
        || ip.is_link_local()                  // 169.254.0.0/16 (includes cloud metadata)
        || ip.is_broadcast()                   // 255.255.255.255
        || ip.is_unspecified()                 // 0.0.0.0
        || ip.is_documentation()              // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
        || ip.octets()[0] == 0                // 0.0.0.0/8
        || ip.octets()[0] >= 224 // Multicast and reserved (224.0.0.0+)
}

/// Check if an IPv6 address is private/internal.
fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()                           // ::1
        || ip.is_unspecified()                 // ::
        || ip.is_multicast()                   // ff00::/8
        // IPv4-mapped addresses (::ffff:0:0/96)
        || ip.to_ipv4_mapped().is_some_and(is_private_ipv4)
        // Unique local (fc00::/7)
        || (ip.segments()[0] & 0xfe00) == 0xfc00
        // Link-local (fe80::/10)
        || (ip.segments()[0] & 0xffc0) == 0xfe80
}

/// Check if an IP address is private/internal.
fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

/// Validate URL for SSRF protection (async DNS resolution).
async fn validate_url_for_ssrf(url: &str) -> Result<()> {
    let parsed = Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?;

    // Only allow http and https schemes
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(Error::InvalidUrl(format!(
                "unsupported scheme '{}', only http/https allowed",
                scheme
            )));
        }
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| Error::InvalidUrl("missing host".to_string()))?;

    // Block obviously dangerous hostnames
    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
        || host_lower == "metadata.google.internal"
    {
        return Err(Error::SsrfBlocked(format!(
            "blocked request to internal host: {}",
            host
        )));
    }

    // Resolve hostname and check all IP addresses (async to avoid blocking runtime)
    let port = parsed.port().unwrap_or(match parsed.scheme() {
        "https" => 443,
        _ => 80,
    });

    let addr_str = format!("{}:{}", host, port);
    if let Ok(addrs) = tokio::net::lookup_host(&addr_str).await {
        for addr in addrs {
            if is_private_ip(addr.ip()) {
                return Err(Error::SsrfBlocked(format!(
                    "blocked request to private IP: {} (resolved from {})",
                    addr.ip(),
                    host
                )));
            }
        }
    }
    // If DNS resolution fails, let reqwest handle it (might be a valid external host)

    Ok(())
}

/// Fetch a URL and return HTTP information.
pub async fn fetch(url: &str, options: &HttpOptions) -> Result<HttpInfo> {
    // SSRF protection: validate URL before making request
    if options.block_private_ips {
        validate_url_for_ssrf(url).await?;
    }

    let client = options.build_client()?;
    let response = client.get(url).send().await?;

    response_to_info(response, options.max_body_size).await
}

/// Convert a reqwest Response to HttpInfo with streaming body size limit.
async fn response_to_info(response: Response, max_body_size: usize) -> Result<HttpInfo> {
    let url = response.url().to_string();
    let status_code = response.status().as_u16();

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            // Extract just the mime type, not charset
            s.split(';').next().unwrap_or(s).trim().to_string()
        });

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.to_string(), v.to_string()))
        })
        .collect();

    // Stream body with size limit - stops downloading when limit reached
    let content_length = response.content_length().unwrap_or(0) as usize;
    let capacity = content_length.min(max_body_size).min(1024 * 1024); // Cap initial alloc at 1MB
    let mut bytes = Vec::with_capacity(capacity);
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let remaining = max_body_size.saturating_sub(bytes.len());
        if remaining == 0 {
            break;
        }
        let to_take = chunk.len().min(remaining);
        bytes.extend_from_slice(&chunk[..to_take]);
        if to_take < chunk.len() {
            break; // Hit the limit
        }
    }

    let body = String::from_utf8_lossy(&bytes).into_owned();

    Ok(HttpInfo {
        url,
        status_code,
        headers,
        content_type,
        redirect_count: 0,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = HttpOptions::default();
        assert!(!options.allow_insecure);
        assert!(options.follow_redirects);
        assert_eq!(options.max_redirects, DEFAULT_MAX_REDIRECTS);
        assert_eq!(options.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert_eq!(options.max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert!(options.block_private_ips);
        assert!(options.user_agent.contains("webpage-info"));
    }

    #[test]
    fn test_builder_pattern() {
        let options = HttpOptions::new()
            .allow_insecure(true)
            .follow_redirects(false)
            .max_redirects(5)
            .timeout(Duration::from_secs(60))
            .max_body_size(1024)
            .block_private_ips(false)
            .user_agent("Custom Agent")
            .header("X-Custom", "Value");

        assert!(options.allow_insecure);
        assert!(!options.follow_redirects);
        assert_eq!(options.max_redirects, 5);
        assert_eq!(options.timeout, Duration::from_secs(60));
        assert_eq!(options.max_body_size, 1024);
        assert!(!options.block_private_ips);
        assert_eq!(options.user_agent, "Custom Agent");
        assert_eq!(options.headers.len(), 1);
    }

    #[tokio::test]
    async fn test_ssrf_blocks_localhost() {
        let result = validate_url_for_ssrf("http://localhost/").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("internal host"));
    }

    #[tokio::test]
    async fn test_ssrf_blocks_private_ip() {
        let result = validate_url_for_ssrf("http://192.168.1.1/").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("private IP"));
    }

    #[tokio::test]
    async fn test_ssrf_blocks_loopback() {
        let result = validate_url_for_ssrf("http://127.0.0.1/").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ssrf_blocks_metadata_endpoint() {
        // AWS/GCP metadata endpoint
        let result = validate_url_for_ssrf("http://169.254.169.254/").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ssrf_blocks_internal_domain() {
        let result = validate_url_for_ssrf("http://server.local/").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ssrf_blocks_file_scheme() {
        let result = validate_url_for_ssrf("file:///etc/passwd").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unsupported scheme")
        );
    }

    #[tokio::test]
    async fn test_ssrf_allows_public_urls() {
        // Note: This test does DNS resolution, so it needs network access
        let result = validate_url_for_ssrf("https://example.com/").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_private_ipv4_detection() {
        assert!(is_private_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
        assert!(is_private_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
        assert!(!is_private_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(Ipv4Addr::new(93, 184, 216, 34)));
    }

    #[test]
    fn test_private_ipv6_detection() {
        assert!(is_private_ipv6(Ipv6Addr::LOCALHOST));
        assert!(is_private_ipv6(Ipv6Addr::UNSPECIFIED));
        // Link-local
        assert!(is_private_ipv6("fe80::1".parse().unwrap()));
        // Unique local
        assert!(is_private_ipv6("fc00::1".parse().unwrap()));
        // Public
        assert!(!is_private_ipv6(
            "2607:f8b0:4004:800::200e".parse().unwrap()
        ));
    }
}
