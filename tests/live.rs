//! Against the live API; ignored by default:
//!
//! ```sh
//! cargo test --test live -- --ignored --nocapture
//! ```

use llms_ink::{CheckTextOptions, Client};

#[tokio::test]
#[ignore = "hits https://llms.ink"]
async fn check_text() {
    let text = "# Example\n\n> One line.\n\n## Docs\n\n- [Guide](/guide.md): start here\n";
    let options = CheckTextOptions {
        base: Some("https://example.com"),
        links: false,
    };
    let report = match Client::builder()
        .key("")
        .build()
        .unwrap()
        .check_text(text, options)
        .await
    {
        Err(err) if matches!(err.status(), Some(404 | 405)) => {
            eprintln!("skipped: POST /api/v0/check isn't deployed yet ({err})");
            return;
        }
        other => other.unwrap(),
    };
    let parsed = report.parsed.expect("parsed");
    assert_eq!(parsed.title.as_deref(), Some("Example"));
    assert_eq!(
        parsed.sections[0].links[0].url,
        "https://example.com/guide.md"
    );
}
