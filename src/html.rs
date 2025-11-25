//! HTML document parsing and metadata extraction

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::Result;
use crate::opengraph::Opengraph;
use crate::schema_org::SchemaOrg;

const FEED_MIME_TYPES: &[&str] = &[
    "application/atom+xml",
    "application/rss+xml",
    "application/json",
    "application/xml",
    "text/xml",
];

// Security limits to prevent DoS via resource exhaustion
const MAX_LINKS: usize = 10_000;
const MAX_SCHEMA_ORG_ITEMS: usize = 100;
const MAX_TEXT_CONTENT_LEN: usize = 1_000_000; // 1 MB of text

fn title_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse("title").unwrap())
}

fn html_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse("html").unwrap())
}

fn meta_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse("meta").unwrap())
}

fn canonical_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse(r#"link[rel="canonical"]"#).unwrap())
}

fn feed_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse(r#"link[rel="alternate"]"#).unwrap())
}

fn body_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse("body").unwrap())
}

fn exclude_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse("script, style, noscript").unwrap())
}

fn link_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse("a[href]").unwrap())
}

fn schema_org_selector() -> &'static Selector {
    static SELECTOR: OnceLock<Selector> = OnceLock::new();
    SELECTOR.get_or_init(|| Selector::parse(r#"script[type="application/ld+json"]"#).unwrap())
}

/// Parsed HTML document information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HtmlInfo {
    /// Document title from `<title>` tag
    pub title: Option<String>,

    /// Meta description
    pub description: Option<String>,

    /// Canonical URL from `<link rel="canonical">`
    pub canonical_url: Option<String>,

    /// RSS/Atom feed URL from `<link rel="alternate" type="application/rss+xml">`
    pub feed_url: Option<String>,

    /// Document language from `<html lang="...">`
    pub language: Option<String>,

    /// Text content extracted from the body (tags stripped)
    pub text_content: String,

    /// All meta tags as key-value pairs
    pub meta: HashMap<String, String>,

    /// OpenGraph metadata
    pub opengraph: Opengraph,

    /// Schema.org structured data (JSON-LD)
    pub schema_org: Vec<SchemaOrg>,

    /// All links found in the document
    pub links: Vec<Link>,
}

/// A link found in the HTML document.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Link {
    /// The URL of the link (resolved if base URL provided)
    pub url: String,

    /// The anchor text of the link
    pub text: String,

    /// The rel attribute if present
    pub rel: Option<String>,
}

impl HtmlInfo {
    /// Parse HTML from a string.
    ///
    /// # Arguments
    /// * `html` - The HTML content to parse
    /// * `base_url` - Optional base URL for resolving relative links
    ///
    /// # Example
    /// ```
    /// use webpage_info::HtmlInfo;
    ///
    /// let html = "<html><head><title>Hello</title></head><body>World</body></html>";
    /// let info = HtmlInfo::from_string(html, None).unwrap();
    /// assert_eq!(info.title, Some("Hello".to_string()));
    /// ```
    pub fn from_string(html: &str, base_url: Option<&str>) -> Result<Self> {
        let base = base_url.and_then(|u| Url::parse(u).ok());
        let document = Html::parse_document(html);
        Ok(Self::extract(&document, base.as_ref()))
    }

    /// Parse HTML from a file.
    ///
    /// # Arguments
    /// * `path` - Path to the HTML file
    /// * `base_url` - Optional base URL for resolving relative links
    pub fn from_file(path: impl AsRef<Path>, base_url: Option<&str>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_string(&content, base_url)
    }

    /// Extract all information from a parsed HTML document.
    fn extract(document: &Html, base_url: Option<&Url>) -> Self {
        let mut info = Self {
            title: Self::extract_title(document),
            language: Self::extract_language(document),
            canonical_url: Self::extract_canonical(document),
            feed_url: Self::extract_feed(document),
            text_content: Self::extract_text_content(document),
            links: Self::extract_links(document, base_url),
            schema_org: Self::extract_schema_org(document),
            ..Default::default()
        };

        // Extract meta tags (sets description, meta, and opengraph)
        info.extract_meta_tags(document);

        info
    }

    fn extract_title(document: &Html) -> Option<String> {
        document
            .select(title_selector())
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn extract_language(document: &Html) -> Option<String> {
        document
            .select(html_selector())
            .next()
            .and_then(|el| el.value().attr("lang"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn extract_meta_tags(&mut self, document: &Html) {
        for element in document.select(meta_selector()) {
            let el = element.value();

            // Get content value
            let content = match el.attr("content") {
                Some(c) => c.trim().to_string(),
                None => {
                    // Handle charset meta tag
                    if let Some(charset) = el.attr("charset") {
                        self.meta.insert("charset".to_string(), charset.to_string());
                    }
                    continue;
                }
            };

            // Get property/name
            let property = el
                .attr("property")
                .or_else(|| el.attr("name"))
                .or_else(|| el.attr("http-equiv"));

            if let Some(prop) = property {
                let prop = prop.trim().to_string();
                self.meta.insert(prop.clone(), content.clone());

                // Handle OpenGraph
                if let Some(og_prop) = prop.strip_prefix("og:") {
                    self.opengraph.extend(og_prop, content.clone());
                }

                // Handle description
                if prop == "description" {
                    self.description = Some(content);
                }
            }
        }
    }

    fn extract_canonical(document: &Html) -> Option<String> {
        document
            .select(canonical_selector())
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn extract_feed(document: &Html) -> Option<String> {
        for element in document.select(feed_selector()) {
            let el = element.value();
            if let Some(link_type) = el.attr("type")
                && FEED_MIME_TYPES.contains(&link_type)
            {
                return el.attr("href").map(|s| s.trim().to_string());
            }
        }
        None
    }

    fn extract_text_content(document: &Html) -> String {
        let Some(body) = document.select(body_selector()).next() else {
            return String::new();
        };

        // Pre-collect excluded node IDs for O(1) lookup instead of O(n) per text node
        let excluded_ids: HashSet<_> = document
            .select(exclude_selector())
            .map(|el| el.id())
            .collect();

        let mut text = String::with_capacity(4096); // Pre-allocate reasonable size

        for node in body.descendants() {
            // Stop if we've reached the size limit
            if text.len() >= MAX_TEXT_CONTENT_LEN {
                break;
            }

            if let Some(text_node) = node.value().as_text() {
                // Check if any ancestor is excluded (O(depth) instead of O(depth * n))
                let is_excluded = node.ancestors().any(|a| excluded_ids.contains(&a.id()));

                if !is_excluded {
                    let trimmed = text_node.trim();
                    if !trimmed.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        // Limit how much we add to stay within bounds
                        let remaining = MAX_TEXT_CONTENT_LEN.saturating_sub(text.len());
                        if trimmed.len() <= remaining {
                            text.push_str(trimmed);
                        } else {
                            text.push_str(&trimmed[..remaining]);
                            break;
                        }
                    }
                }
            }
        }

        text
    }

    fn extract_links(document: &Html, base_url: Option<&Url>) -> Vec<Link> {
        document
            .select(link_selector())
            .filter_map(|element| {
                let href = element.value().attr("href")?;
                let href = href.trim();

                // Skip empty and javascript: links
                if href.is_empty() || href.starts_with("javascript:") {
                    return None;
                }

                let url = if let Some(base) = base_url {
                    base.join(href)
                        .map(|u| u.to_string())
                        .unwrap_or_else(|_| href.to_string())
                } else {
                    href.to_string()
                };

                let text = element.text().collect::<String>().trim().to_string();
                let rel = element.value().attr("rel").map(|s| s.to_string());

                Some(Link { url, text, rel })
            })
            .take(MAX_LINKS)
            .collect()
    }

    fn extract_schema_org(document: &Html) -> Vec<SchemaOrg> {
        document
            .select(schema_org_selector())
            .flat_map(|element| {
                let content = element.text().collect::<String>();
                SchemaOrg::parse(&content)
            })
            .take(MAX_SCHEMA_ORG_ITEMS)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let html = r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <title>Test Page</title>
                <meta name="description" content="A test page">
                <meta property="og:title" content="OG Title">
                <meta property="og:type" content="article">
                <link rel="canonical" href="https://example.com/test">
            </head>
            <body>
                <p>Hello World</p>
                <a href="/about">About Us</a>
            </body>
            </html>
        "#;

        let info = HtmlInfo::from_string(html, Some("https://example.com/")).unwrap();

        assert_eq!(info.title, Some("Test Page".to_string()));
        assert_eq!(info.description, Some("A test page".to_string()));
        assert_eq!(info.language, Some("en".to_string()));
        assert_eq!(
            info.canonical_url,
            Some("https://example.com/test".to_string())
        );
        assert_eq!(info.opengraph.title, Some("OG Title".to_string()));
        assert_eq!(info.opengraph.og_type, Some("article".to_string()));
        assert!(info.text_content.contains("Hello World"));
        assert_eq!(info.links.len(), 1);
        assert_eq!(info.links[0].url, "https://example.com/about");
        assert_eq!(info.links[0].text, "About Us");
    }

    #[test]
    fn test_feed_extraction() {
        let html = r#"
            <html>
            <head>
                <link rel="alternate" type="application/rss+xml" href="/feed.xml">
            </head>
            </html>
        "#;

        let info = HtmlInfo::from_string(html, None).unwrap();
        assert_eq!(info.feed_url, Some("/feed.xml".to_string()));
    }

    #[test]
    fn test_schema_org_extraction() {
        let html = r#"
            <html>
            <head>
                <script type="application/ld+json">
                {"@type": "Article", "headline": "Test Article"}
                </script>
            </head>
            </html>
        "#;

        let info = HtmlInfo::from_string(html, None).unwrap();
        assert_eq!(info.schema_org.len(), 1);
        assert_eq!(info.schema_org[0].schema_type, "Article");
    }

    #[test]
    fn test_text_excludes_scripts() {
        let html = r#"
            <html>
            <body>
                <p>Visible text</p>
                <script>console.log('hidden');</script>
                <style>.hidden { display: none; }</style>
                <p>More visible</p>
            </body>
            </html>
        "#;

        let info = HtmlInfo::from_string(html, None).unwrap();
        assert!(info.text_content.contains("Visible text"));
        assert!(info.text_content.contains("More visible"));
        assert!(!info.text_content.contains("console.log"));
        assert!(!info.text_content.contains(".hidden"));
    }
}
