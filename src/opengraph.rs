//! OpenGraph metadata extraction
//!
//! Parses [OpenGraph](https://ogp.me/) protocol metadata from HTML documents.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// Security limit for media collections
const MAX_MEDIA_ITEMS: usize = 100;

/// OpenGraph metadata for a webpage.
///
/// OpenGraph is a protocol for structured data in web pages, originally
/// developed by Facebook. It allows websites to control how content appears
/// when shared on social media platforms.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Opengraph {
    /// The type of object (e.g., "website", "article", "video.movie")
    pub og_type: Option<String>,

    /// The title of the object
    pub title: Option<String>,

    /// A brief description of the content
    pub description: Option<String>,

    /// The canonical URL of the object
    pub url: Option<String>,

    /// The name of the site
    pub site_name: Option<String>,

    /// The locale of the content (e.g., "en_US")
    pub locale: Option<String>,

    /// Alternative locales available
    pub locale_alternates: Vec<String>,

    /// Images associated with the object
    pub images: Vec<OpengraphMedia>,

    /// Videos associated with the object
    pub videos: Vec<OpengraphMedia>,

    /// Audio files associated with the object
    pub audios: Vec<OpengraphMedia>,

    /// Additional properties not covered by standard fields
    pub properties: HashMap<String, String>,
}

/// Media object (image, video, or audio) in OpenGraph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpengraphMedia {
    /// URL of the media
    pub url: String,

    /// Secure (HTTPS) URL of the media
    pub secure_url: Option<String>,

    /// MIME type (e.g., "image/jpeg")
    pub mime_type: Option<String>,

    /// Width in pixels
    pub width: Option<u32>,

    /// Height in pixels
    pub height: Option<u32>,

    /// Alternative text description
    pub alt: Option<String>,

    /// Additional properties
    pub properties: HashMap<String, String>,
}

impl OpengraphMedia {
    /// Create a new media object with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }
}

impl Opengraph {
    /// Create an empty OpenGraph structure.
    pub fn new() -> Self {
        Self::default()
    }

    /// Extend the OpenGraph data with a property and its content.
    ///
    /// Property names should be without the "og:" prefix (e.g., "title" not "og:title").
    pub fn extend(&mut self, property: &str, content: String) {
        match property {
            "type" => self.og_type = Some(content),
            "title" => self.title = Some(content),
            "description" => self.description = Some(content),
            "url" => self.url = Some(content),
            "site_name" => self.site_name = Some(content),
            "locale" => self.locale = Some(content),
            "locale:alternate" => self.locale_alternates.push(content),
            _ if property.starts_with("image") => {
                Self::extend_media("image", property, content, &mut self.images);
            }
            _ if property.starts_with("video") => {
                Self::extend_media("video", property, content, &mut self.videos);
            }
            _ if property.starts_with("audio") => {
                Self::extend_media("audio", property, content, &mut self.audios);
            }
            _ => {
                self.properties.insert(property.to_string(), content);
            }
        }
    }

    /// Parse media properties (image, video, audio).
    fn extend_media(
        media_type: &str,
        property: &str,
        content: String,
        collection: &mut Vec<OpengraphMedia>,
    ) {
        // "image" or "image:url" starts a new image
        if property == media_type || property.strip_prefix(media_type) == Some(":url") {
            // Enforce limit to prevent resource exhaustion
            if collection.len() < MAX_MEDIA_ITEMS {
                collection.push(OpengraphMedia::new(content));
            }
            return;
        }

        // Other properties modify the last media item
        if let Some(media) = collection.last_mut() {
            // Avoid allocation: check prefix without format!()
            let prefix_len = media_type.len() + 1; // "image:" length
            let suffix = if property.len() > prefix_len
                && property.starts_with(media_type)
                && property.as_bytes().get(media_type.len()) == Some(&b':')
            {
                &property[prefix_len..]
            } else {
                ""
            };

            match suffix {
                "secure_url" => media.secure_url = Some(content),
                "type" => media.mime_type = Some(content),
                "width" => media.width = content.parse().ok(),
                "height" => media.height = content.parse().ok(),
                "alt" => media.alt = Some(content),
                "" => {}
                _ => {
                    media.properties.insert(suffix.to_string(), content);
                }
            }
        }
    }

    /// Check if the OpenGraph data is empty (no meaningful content).
    pub fn is_empty(&self) -> bool {
        self.og_type.is_none()
            && self.title.is_none()
            && self.description.is_none()
            && self.url.is_none()
            && self.images.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_properties() {
        let mut og = Opengraph::new();
        og.extend("type", "article".to_string());
        og.extend("title", "Test Article".to_string());
        og.extend("description", "A test description".to_string());

        assert_eq!(og.og_type, Some("article".to_string()));
        assert_eq!(og.title, Some("Test Article".to_string()));
        assert_eq!(og.description, Some("A test description".to_string()));
    }

    #[test]
    fn test_image_with_properties() {
        let mut og = Opengraph::new();
        og.extend("image", "http://example.org/image.png".to_string());
        og.extend(
            "image:secure_url",
            "https://example.org/image.png".to_string(),
        );
        og.extend("image:width", "800".to_string());
        og.extend("image:height", "600".to_string());
        og.extend("image:alt", "Example image".to_string());

        assert_eq!(og.images.len(), 1);
        let image = &og.images[0];
        assert_eq!(image.url, "http://example.org/image.png");
        assert_eq!(
            image.secure_url,
            Some("https://example.org/image.png".to_string())
        );
        assert_eq!(image.width, Some(800));
        assert_eq!(image.height, Some(600));
        assert_eq!(image.alt, Some("Example image".to_string()));
    }

    #[test]
    fn test_multiple_images() {
        let mut og = Opengraph::new();
        og.extend("image", "http://example.org/image1.png".to_string());
        og.extend("image:width", "100".to_string());
        og.extend("image", "http://example.org/image2.png".to_string());
        og.extend("image:width", "200".to_string());

        assert_eq!(og.images.len(), 2);
        assert_eq!(og.images[0].url, "http://example.org/image1.png");
        assert_eq!(og.images[0].width, Some(100));
        assert_eq!(og.images[1].url, "http://example.org/image2.png");
        assert_eq!(og.images[1].width, Some(200));
    }

    #[test]
    fn test_is_empty() {
        let og = Opengraph::new();
        assert!(og.is_empty());

        let mut og2 = Opengraph::new();
        og2.extend("title", "Test".to_string());
        assert!(!og2.is_empty());
    }
}
