#![cfg(feature = "blocking")]

mod common;

use std::time::Duration;

use common::{json, problem, serve, CHECK, CHECK_TEXT, LLMS};
use llms_ink::blocking::Client;
use llms_ink::{CheckTextOptions, Error, Kind, LlmsOptions, Mode};

fn client(base_url: &str, key: &str) -> Client {
    Client::builder()
        .base_url(base_url)
        .key(key)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap()
}

#[test]
fn llms_and_check() {
    let stub = serve(vec![json(LLMS), json(CHECK)]);
    let client = client(&stub.base_url, "lk_test");
    let options = LlmsOptions {
        mode: Mode::Full,
        fresh: false,
    };
    let result = client.llms("youtube.com/watch?v=abc", options).unwrap();
    let head = stub.request();
    assert!(
        head.starts_with("get /api/v0/llms?url=youtube.com%2fwatch%3fv%3dabc&mode=full http/1.1"),
        "{head}"
    );
    assert!(head.contains("authorization: bearer lk_test\r\n"), "{head}");
    assert_eq!(result.kind, Kind::Video);

    let report = client.check("llmstxt.org").unwrap();
    assert!(stub
        .request()
        .starts_with("get /api/v0/check?url=llmstxt.org "));
    assert_eq!(report.sections, 2);
}

#[test]
fn check_text() {
    let stub = serve(vec![json(CHECK_TEXT)]);
    let options = CheckTextOptions {
        base: Some("https://example.com"),
        links: false,
    };
    let report = client(&stub.base_url, "")
        .check_text("# Example\n", options)
        .unwrap();
    let request = stub.request();
    assert!(
        request.starts_with("post /api/v0/check?base=https%3a%2f%2fexample.com http/1.1"),
        "{request}"
    );
    assert!(request.ends_with("\r\n\r\n# Example\n"), "{request}");
    assert_eq!(report.parsed.unwrap().sections[0].links[0].line, 7);
}

#[test]
fn errors() {
    let stub = serve(vec![
        problem(
            "401 Unauthorized",
            "",
            r#"{"title":"Unauthorized","status":401,"code":"invalid_key","detail":"Unknown API key."}"#,
        ),
        problem(
            "429 Too Many Requests",
            "retry-after: 7\r\n",
            r#"{"title":"Too Many Requests","status":429,"code":"quota_exceeded","detail":"Daily quota used up."}"#,
        ),
    ]);
    let client = client(&stub.base_url, "lk_test");
    let err = client.check("llmstxt.org").unwrap_err();
    assert!(matches!(err, Error::InvalidKey { .. }), "{err:?}");
    let err = client.check("llmstxt.org").unwrap_err();
    assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
    assert!(matches!(err, Error::QuotaExceeded { .. }), "{err:?}");
}
