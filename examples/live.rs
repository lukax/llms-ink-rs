//! Against the live API: llms.txt and the checker for a link, then a bad key.
//!
//! ```sh
//! cargo run --example live -- llmstxt.org
//! ```

use llms_ink::{Client, Error, LlmsOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "llmstxt.org".into());
    let client = Client::new();

    let index = client.llms(&url, LlmsOptions::default()).await?;
    println!(
        "llms: {} | {:?}/{:?} | {} pages, {} tokens, cached: {}",
        index.title, index.kind, index.source, index.pages, index.tokens, index.cached
    );
    println!(
        "{}",
        index.text.lines().take(3).collect::<Vec<_>>().join("\n")
    );

    let report = client.check(&url).await?;
    println!(
        "check: {} found: {}, {} errors, {} warnings, {} links ({} broken)",
        report.url.as_deref().unwrap_or(&url),
        report.found,
        report.summary.errors,
        report.summary.warnings,
        report.links.total,
        report.links.broken.len()
    );

    let bad = Client::builder()
        .key("lk_00000000000000000000000000000000")
        .build()?;
    match bad.llms(&url, LlmsOptions::default()).await {
        Err(Error::InvalidKey { detail }) => println!("bad key: 401 invalid_key: {detail}"),
        other => return Err(format!("expected Error::InvalidKey, got {other:?}").into()),
    }
    Ok(())
}
