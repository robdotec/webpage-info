# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-11-25

### Added

- Initial release of webpage-info
- HTML parsing with metadata extraction
  - Document title from `<title>` tag
  - Meta description and other meta tags
  - Language detection from `<html lang="...">`
  - Canonical URL from `<link rel="canonical">`
  - RSS/Atom feed URL detection
  - Text content extraction (excluding scripts/styles)
  - Link extraction with URL resolution
- OpenGraph metadata extraction
  - Standard properties (type, title, description, url, site_name, locale)
  - Image, video, and audio media with properties
  - Locale alternates
- Schema.org JSON-LD parsing
  - Single object and array support
  - `@graph` structure support
  - Helper methods for accessing properties
- Async HTTP fetching with reqwest
  - Configurable timeout, redirects, and headers
  - TLS support via rustls
  - Gzip and Brotli compression
- Security features
  - SSRF protection (blocks private IPs by default)
  - Response body size limits (default 10 MB)
  - Collection limits to prevent resource exhaustion
  - Async DNS resolution to avoid blocking
- Performance optimizations
  - Cached CSS selectors via `OnceLock`
  - O(1) excluded element lookup with `HashSet`
  - Streaming response body reading
  - Zero-allocation string operations where possible

### Security

- Blocks requests to localhost, private networks, and link-local addresses
- Blocks dangerous URL schemes (only http/https allowed)
- Limits response body size to prevent memory exhaustion
- Limits extracted collections (links, schema.org items, media)
- Limits text content extraction length

## [Unreleased]

- No changes yet
