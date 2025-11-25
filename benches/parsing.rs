//! Benchmarks for HTML parsing and metadata extraction

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use webpage_info::HtmlInfo;

const SAMPLE_HTML: &str = include_str!("../test_data/sample.html");

fn bench_full_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("html_parsing");
    group.throughput(Throughput::Bytes(SAMPLE_HTML.len() as u64));

    group.bench_function("full_parse", |b| {
        b.iter(|| HtmlInfo::from_string(black_box(SAMPLE_HTML), None))
    });

    group.bench_function("full_parse_with_base_url", |b| {
        b.iter(|| {
            HtmlInfo::from_string(
                black_box(SAMPLE_HTML),
                Some("https://example.com/articles/"),
            )
        })
    });

    group.finish();
}

fn bench_document_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_sizes");

    // Minimal HTML
    let minimal =
        r#"<!DOCTYPE html><html><head><title>Test</title></head><body><p>Hello</p></body></html>"#;

    // Small HTML with some metadata
    let small = r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <title>Small Page</title>
            <meta name="description" content="A small test page">
            <meta property="og:title" content="Small Page">
        </head>
        <body>
            <h1>Hello World</h1>
            <p>This is a small test page.</p>
            <a href="/link1">Link 1</a>
            <a href="/link2">Link 2</a>
        </body>
        </html>"#;

    // Medium = sample.html
    let medium = SAMPLE_HTML;

    // Large = sample.html repeated with more links
    let large = generate_large_html(100);

    for (name, html) in [
        ("minimal", minimal.to_string()),
        ("small", small.to_string()),
        ("medium", medium.to_string()),
        ("large", large),
    ] {
        group.throughput(Throughput::Bytes(html.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), &html, |b, html| {
            b.iter(|| HtmlInfo::from_string(black_box(html), None))
        });
    }

    group.finish();
}

fn bench_link_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_extraction");

    // Generate HTML with varying numbers of links
    for link_count in [10, 100, 500, 1000] {
        let html = generate_html_with_links(link_count);
        group.throughput(Throughput::Elements(link_count as u64));
        group.bench_with_input(
            BenchmarkId::new("extract_links", link_count),
            &html,
            |b, html| {
                b.iter(|| HtmlInfo::from_string(black_box(html), Some("https://example.com/")))
            },
        );
    }

    group.finish();
}

fn bench_text_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_extraction");

    // Generate HTML with varying amounts of text and script tags
    for (paragraphs, scripts) in [(10, 2), (50, 10), (100, 20)] {
        let html = generate_html_with_text(paragraphs, scripts);
        let label = format!("{}p_{}s", paragraphs, scripts);
        group.bench_with_input(
            BenchmarkId::new("extract_text", &label),
            &html,
            |b, html| b.iter(|| HtmlInfo::from_string(black_box(html), None)),
        );
    }

    group.finish();
}

fn bench_schema_org_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_org");

    // Simple schema
    let simple_schema = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {"@type": "Article", "headline": "Test"}
            </script>
        </head>
        <body></body>
        </html>"#;

    // Complex schema with @graph
    let complex_schema = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@graph": [
                    {"@type": "Organization", "name": "Test Org", "url": "https://example.com"},
                    {"@type": "WebSite", "name": "Test Site", "url": "https://example.com"},
                    {"@type": "Article", "headline": "Test Article", "author": {"@type": "Person", "name": "John"}},
                    {"@type": "BreadcrumbList", "itemListElement": [{"@type": "ListItem", "position": 1, "name": "Home"}]}
                ]
            }
            </script>
        </head>
        <body></body>
        </html>"#;

    group.bench_function("simple", |b| {
        b.iter(|| HtmlInfo::from_string(black_box(simple_schema), None))
    });

    group.bench_function("complex_graph", |b| {
        b.iter(|| HtmlInfo::from_string(black_box(complex_schema), None))
    });

    group.finish();
}

fn bench_opengraph_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("opengraph");

    // Minimal OG
    let minimal_og = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="Test">
            <meta property="og:type" content="website">
        </head>
        <body></body>
        </html>"#;

    // Full OG with multiple images
    let full_og = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="Full OpenGraph Test">
            <meta property="og:type" content="article">
            <meta property="og:description" content="A comprehensive OpenGraph test">
            <meta property="og:url" content="https://example.com/test">
            <meta property="og:site_name" content="Example Site">
            <meta property="og:locale" content="en_US">
            <meta property="og:image" content="https://example.com/img1.jpg">
            <meta property="og:image:width" content="1200">
            <meta property="og:image:height" content="630">
            <meta property="og:image" content="https://example.com/img2.jpg">
            <meta property="og:image:width" content="800">
            <meta property="og:image:height" content="600">
            <meta property="og:image" content="https://example.com/img3.jpg">
            <meta property="og:video" content="https://example.com/video.mp4">
            <meta property="og:video:type" content="video/mp4">
            <meta property="og:video:width" content="1920">
            <meta property="og:video:height" content="1080">
        </head>
        <body></body>
        </html>"#;

    group.bench_function("minimal", |b| {
        b.iter(|| HtmlInfo::from_string(black_box(minimal_og), None))
    });

    group.bench_function("full_with_images", |b| {
        b.iter(|| HtmlInfo::from_string(black_box(full_og), None))
    });

    group.finish();
}

// Helper functions to generate test HTML

fn generate_large_html(link_count: usize) -> String {
    let mut html = String::from(SAMPLE_HTML);
    // Insert additional links before </body>
    let links: String = (0..link_count)
        .map(|i| format!(r#"<a href="/page/{}">Page {}</a>"#, i, i))
        .collect::<Vec<_>>()
        .join("\n");
    html = html.replace("</body>", &format!("<div>{}</div></body>", links));
    html
}

fn generate_html_with_links(count: usize) -> String {
    let links: String = (0..count)
        .map(|i| format!(r#"<a href="/link/{}">Link number {}</a>"#, i, i))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
        <html>
        <head><title>Links Test</title></head>
        <body>
            <nav>{}</nav>
        </body>
        </html>"#,
        links
    )
}

fn generate_html_with_text(paragraphs: usize, scripts: usize) -> String {
    let text: String = (0..paragraphs)
        .map(|i| {
            format!(
                "<p>This is paragraph number {}. It contains some text that should be extracted \
                 during parsing. The text extraction algorithm needs to handle various amounts of \
                 content efficiently.</p>",
                i
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let scripts_html: String = (0..scripts)
        .map(|i| {
            format!(
                "<script>console.log('Script {}'); var data{} = {{}};</script>",
                i, i
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
        <html>
        <head><title>Text Test</title></head>
        <body>
            {}
            {}
            <style>.test {{ display: none; }} .another {{ color: red; }}</style>
        </body>
        </html>"#,
        text, scripts_html
    )
}

criterion_group!(
    benches,
    bench_full_parsing,
    bench_document_sizes,
    bench_link_extraction,
    bench_text_extraction,
    bench_schema_org_parsing,
    bench_opengraph_parsing,
);

criterion_main!(benches);
