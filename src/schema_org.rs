//! Schema.org structured data extraction
//!
//! Parses [Schema.org](https://schema.org/) JSON-LD structured data from HTML documents.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema.org structured data item.
///
/// Schema.org provides a collection of shared vocabularies that webmasters can use
/// to mark up their pages in ways that can be understood by major search engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaOrg {
    /// The @type of the schema (e.g., "Article", "Product", "Organization")
    pub schema_type: String,

    /// The full JSON-LD value containing all properties
    pub value: Value,
}

impl SchemaOrg {
    /// Parse Schema.org data from a JSON-LD string.
    ///
    /// Returns a vector of SchemaOrg items found in the JSON-LD content.
    /// Handles both single objects and arrays, as well as @graph structures.
    pub fn parse(content: &str) -> Vec<Self> {
        let Ok(node) = serde_json::from_str::<Value>(content) else {
            return Vec::new();
        };

        Self::extract_from_value(node)
    }

    /// Extract Schema.org items from a parsed JSON value.
    fn extract_from_value(node: Value) -> Vec<Self> {
        // Convert single object to array for uniform handling, taking ownership
        let values = match node {
            Value::Array(arr) => arr,
            Value::Object(mut obj) => {
                // Check for @graph structure - take ownership instead of cloning
                if let Some(Value::Array(graph)) = obj.remove("@graph") {
                    graph
                } else {
                    vec![Value::Object(obj)]
                }
            }
            _ => return Vec::new(),
        };

        values
            .into_iter()
            .filter_map(|v| {
                let schema_type = match &v["@type"] {
                    Value::String(s) => s.clone(),
                    Value::Array(arr) => {
                        // Handle multiple types - take the first one
                        arr.first()
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())?
                    }
                    _ => return None,
                };

                Some(SchemaOrg {
                    schema_type,
                    value: v,
                })
            })
            .collect()
    }

    /// Get a property value from the schema as a string.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.value.get(key).and_then(|v| v.as_str())
    }

    /// Get a property value from the schema as an i64.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.value.get(key).and_then(|v| v.as_i64())
    }

    /// Get a property value from the schema as a nested object.
    pub fn get_object(&self, key: &str) -> Option<&Value> {
        self.value.get(key).filter(|v| v.is_object())
    }

    /// Get a property value from the schema as an array.
    pub fn get_array(&self, key: &str) -> Option<&Vec<Value>> {
        self.value.get(key).and_then(|v| v.as_array())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_object() {
        let schema = SchemaOrg::parse("{}");
        assert!(schema.is_empty());
    }

    #[test]
    fn test_invalid_json() {
        let schema = SchemaOrg::parse("not json");
        assert!(schema.is_empty());
    }

    #[test]
    fn test_single_type() {
        let schema = SchemaOrg::parse(r#"{"@type": "NewsArticle", "headline": "Test"}"#);
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].schema_type, "NewsArticle");
        assert_eq!(schema[0].get_str("headline"), Some("Test"));
    }

    #[test]
    fn test_array_of_types() {
        let schema = SchemaOrg::parse(r#"[{"@type": "Article"}, {"@type": "WebPage"}]"#);
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].schema_type, "Article");
        assert_eq!(schema[1].schema_type, "WebPage");
    }

    #[test]
    fn test_graph_structure() {
        let json = r#"{
            "@context": "https://schema.org",
            "@graph": [
                {"@type": "Organization", "name": "Example"},
                {"@type": "WebSite", "url": "https://example.org"}
            ]
        }"#;
        let schema = SchemaOrg::parse(json);
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].schema_type, "Organization");
        assert_eq!(schema[0].get_str("name"), Some("Example"));
        assert_eq!(schema[1].schema_type, "WebSite");
    }

    #[test]
    fn test_multiple_types() {
        let schema = SchemaOrg::parse(r#"{"@type": ["Article", "BlogPosting"]}"#);
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].schema_type, "Article");
    }

    #[test]
    fn test_helper_methods() {
        let schema = SchemaOrg::parse(
            r#"{
            "@type": "Product",
            "name": "Widget",
            "price": 99,
            "offers": {"@type": "Offer"},
            "images": ["a.jpg", "b.jpg"]
        }"#,
        );

        let product = &schema[0];
        assert_eq!(product.get_str("name"), Some("Widget"));
        assert_eq!(product.get_i64("price"), Some(99));
        assert!(product.get_object("offers").is_some());
        assert_eq!(product.get_array("images").map(|a| a.len()), Some(2));
    }
}
