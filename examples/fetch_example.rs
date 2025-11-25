//! Example: Fetch and display webpage metadata from example.com

use webpage_info::WebpageInfo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com";

    println!("Fetching: {}\n", url);

    let info = WebpageInfo::fetch(url).await?;

    // HTTP Info
    println!("=== HTTP Info ===");
    println!("Final URL: {}", info.http.url);
    println!("Status: {}", info.http.status_code);
    println!("Content-Type: {:?}", info.http.content_type);
    println!("Headers: {} total", info.http.headers.len());

    // HTML Info
    println!("\n=== HTML Info ===");
    println!("Title: {:?}", info.html.title);
    println!("Description: {:?}", info.html.description);
    println!("Language: {:?}", info.html.language);
    println!("Canonical URL: {:?}", info.html.canonical_url);
    println!("Feed URL: {:?}", info.html.feed_url);

    // Text content (truncated)
    let text_preview = if info.html.text_content.len() > 100 {
        format!("{}...", &info.html.text_content[..100])
    } else {
        info.html.text_content.clone()
    };
    println!("Text content: {}", text_preview);

    // Meta tags
    println!("\n=== Meta Tags ({}) ===", info.html.meta.len());
    for (key, value) in &info.html.meta {
        let display_value = if value.len() > 50 {
            format!("{}...", &value[..50])
        } else {
            value.clone()
        };
        println!("  {}: {}", key, display_value);
    }

    // OpenGraph
    println!("\n=== OpenGraph ===");
    println!("Type: {:?}", info.html.opengraph.og_type);
    println!("Title: {:?}", info.html.opengraph.title);
    println!("Description: {:?}", info.html.opengraph.description);
    println!("Images: {}", info.html.opengraph.images.len());
    for (i, img) in info.html.opengraph.images.iter().enumerate() {
        println!(
            "  [{}] {} ({}x{:?})",
            i,
            img.url,
            img.width.unwrap_or(0),
            img.height
        );
    }

    // Schema.org
    println!("\n=== Schema.org ({}) ===", info.html.schema_org.len());
    for schema in &info.html.schema_org {
        println!("  Type: {}", schema.schema_type);
    }

    // Links
    println!("\n=== Links ({}) ===", info.html.links.len());
    for link in info.html.links.iter().take(5) {
        println!("  {} -> {}", link.text, link.url);
    }
    if info.html.links.len() > 5 {
        println!("  ... and {} more", info.html.links.len() - 5);
    }

    Ok(())
}
