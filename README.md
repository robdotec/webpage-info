# webpage-info

A fast, safe Rust library for extracting metadata from web pages. Parses HTML to extract titles, descriptions, OpenGraph data, Schema.org JSON-LD, links, and more.

[![Crates.io](https://img.shields.io/crates/v/webpage-info.svg)](https://crates.io/crates/webpage-info)
[![Documentation](https://docs.rs/webpage-info/badge.svg)](https://docs.rs/webpage-info)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Fast HTML parsing** with cached CSS selectors (~92 MiB/s throughput)
- **OpenGraph metadata** extraction (og:title, og:image, etc.)
- **Schema.org JSON-LD** structured data parsing
- **Link extraction** with URL resolution
- **Text content extraction** excluding scripts and styles
- **Async HTTP fetching** with security protections
- **SSRF protection** blocks requests to private IPs by default
- **Resource limits** prevent memory exhaustion attacks

## Installation

```toml
[dependencies]
webpage-info = "1.0"
```

For HTML parsing only (no HTTP client):

```toml
[dependencies]
webpage-info = { version = "1.0", default-features = false }
```

## Quick Start

### Fetch and parse a URL

```rust
use webpage_info::WebpageInfo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let info = WebpageInfo::fetch("https://example.com").await?;

    println!("Title: {:?}", info.html.title);
    println!("Description: {:?}", info.html.description);
    println!("OpenGraph: {:?}", info.html.opengraph.title);
    println!("Links: {}", info.html.links.len());

    Ok(())
}
```

### Parse HTML directly

```rust
use webpage_info::HtmlInfo;

let html = r#"
    <html>
    <head>
        <title>My Page</title>
        <meta property="og:title" content="OpenGraph Title">
    </head>
    <body>
        <a href="/about">About</a>
    </body>
    </html>
"#;

let info = HtmlInfo::from_string(html, Some("https://example.com/"))?;

assert_eq!(info.title, Some("My Page".to_string()));
assert_eq!(info.opengraph.title, Some("OpenGraph Title".to_string()));
assert_eq!(info.links[0].url, "https://example.com/about");
```

### Custom HTTP options

```rust
use webpage_info::{WebpageInfo, HttpOptions};
use std::time::Duration;

let options = HttpOptions::new()
    .timeout(Duration::from_secs(10))
    .max_body_size(5 * 1024 * 1024)  // 5 MB
    .user_agent("MyBot/1.0");

let info = WebpageInfo::fetch_with_options("https://example.com", options).await?;
```

## Extracted Data

### HtmlInfo

| Field | Type | Description |
|-------|------|-------------|
| `title` | `Option<String>` | Document title from `<title>` tag |
| `description` | `Option<String>` | Meta description |
| `language` | `Option<String>` | Language from `<html lang="...">` |
| `canonical_url` | `Option<String>` | Canonical URL from `<link rel="canonical">` |
| `feed_url` | `Option<String>` | RSS/Atom feed URL |
| `text_content` | `String` | Extracted text (scripts/styles excluded) |
| `meta` | `HashMap<String, String>` | All meta tags |
| `opengraph` | `Opengraph` | OpenGraph metadata |
| `schema_org` | `Vec<SchemaOrg>` | Schema.org JSON-LD data |
| `links` | `Vec<Link>` | All links in the document |

### OpenGraph

```rust
let og = &info.opengraph;
println!("Type: {:?}", og.og_type);      // "article", "website", etc.
println!("Title: {:?}", og.title);
println!("Description: {:?}", og.description);
println!("Site: {:?}", og.site_name);
println!("Images: {:?}", og.images);     // Vec<OpengraphMedia>
println!("Videos: {:?}", og.videos);
```

### Schema.org

```rust
for schema in &info.schema_org {
    println!("Type: {}", schema.schema_type);  // "Article", "Product", etc.
    println!("Name: {:?}", schema.get_str("name"));
    println!("Full JSON: {}", schema.value);
}
```

## Security

### SSRF Protection

By default, requests to private/internal IP addresses are blocked:

- Localhost (`127.0.0.1`, `::1`)
- Private networks (`10.x`, `172.16-31.x`, `192.168.x`)
- Link-local (`169.254.x` - includes cloud metadata endpoints)
- Internal hostnames (`.local`, `.internal`)

To disable (not recommended for user-supplied URLs):

```rust
let options = HttpOptions::new().block_private_ips(false);
```

### Resource Limits

Default limits prevent resource exhaustion:

| Limit | Default | Option |
|-------|---------|--------|
| Response body | 10 MB | `max_body_size()` |
| Links | 10,000 | - |
| Schema.org items | 100 | - |
| Text content | 1 MB | - |
| OpenGraph media | 100 each | - |

## Performance

Benchmarks on sample HTML (9KB document):

| Operation | Time | Throughput |
|-----------|------|------------|
| Full parse | ~96 µs | 92 MiB/s |
| 1000 links | ~725 µs | 1.4M links/s |
| Text extraction | ~59 µs | - |
| Schema.org (complex) | ~6 µs | - |

Run benchmarks:

```bash
cargo bench
```

## Examples

```bash
# Fetch and display webpage info
cargo run --example fetch_example
```

## License

MIT License - see [LICENSE](LICENSE) for details.
