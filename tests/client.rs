mod common;

use std::time::Duration;

use common::{json, problem, response, serve, CHECK, LLMS};
use llms_ink::{Client, Error, IssueLevel, Kind, LlmsOptions, Mode, Source};

fn client(base_url: &str, key: &str) -> Client {
    Client::builder()
        .base_url(format!("{base_url}/"))
        .key(key)
        .build()
        .unwrap()
}

#[tokio::test]
async fn llms_sends_query_key_and_user_agent_and_decodes_the_result() {
    let stub = serve(vec![json(LLMS)]);
    let options = LlmsOptions {
        mode: Mode::Full,
        fresh: true,
    };
    let result = client(&stub.base_url, "lk_test")
        .llms("youtube.com/watch?v=abc&t=1", options)
        .await
        .unwrap();

    let head = stub.request();
    assert!(
        head.starts_with(
            "get /api/v0/llms?url=youtube.com%2fwatch%3fv%3dabc%26t%3d1&mode=full&fresh=1 http/1.1"
        ),
        "{head}"
    );
    assert!(head.contains("authorization: bearer lk_test\r\n"), "{head}");
    assert!(head.contains("accept: application/json\r\n"), "{head}");
    assert!(
        head.contains(&format!(
            "user-agent: llms-ink-rust/{}\r\n",
            env!("CARGO_PKG_VERSION")
        )),
        "{head}"
    );

    assert_eq!(result.kind, Kind::Video);
    assert_eq!(result.source, Source::Generated);
    assert_eq!(result.mode, Mode::Full);
    assert_eq!(result.tokens, 1234);
    assert_eq!(result.model, None);
    let media = result.media.unwrap();
    assert_eq!(media.duration, Some(61.5));
    assert_eq!(media.downloads, ["mp4", "m4a"]);
    assert_eq!(media.is_live, None);
    assert_eq!(media.transcript.unwrap().segments[0].text, "Hello");
}

#[tokio::test]
async fn default_options_ask_for_the_index_anonymously() {
    let stub = serve(vec![json(&LLMS.replace("\"full\"", "\"llms\""))]);
    let client = client(&stub.base_url, "");
    assert!(!client.has_key());
    let result = client
        .llms("llmstxt.org", LlmsOptions::default())
        .await
        .unwrap();
    let head = stub.request();
    assert!(
        head.starts_with("get /api/v0/llms?url=llmstxt.org&mode=llms http/1.1"),
        "{head}"
    );
    assert!(!head.contains("authorization:"), "{head}");
    assert_eq!(result.mode, Mode::Llms);
}

#[tokio::test]
async fn check_decodes_the_report_and_tolerates_unknown_levels() {
    let stub = serve(vec![json(CHECK)]);
    let report = client(&stub.base_url, "")
        .check("llmstxt.org")
        .await
        .unwrap();
    assert!(stub
        .request()
        .starts_with("get /api/v0/check?url=llmstxt.org http/1.1"));
    assert!(report.found && report.llms_full);
    assert_eq!(report.links.broken[0].status, 404);
    assert_eq!(report.issues[0].level, IssueLevel::Error);
    assert_eq!(report.issues[0].line, Some(12));
    assert_eq!(report.issues[1].level, IssueLevel::Unknown);
    assert_eq!(report.summary.errors, 1);
}

#[tokio::test]
async fn invalid_key() {
    let stub = serve(vec![problem(
        "401 Unauthorized",
        "",
        r#"{"type":"https://llms.ink/docs/#errors","title":"Unauthorized","status":401,"code":"invalid_key","detail":"Unknown API key."}"#,
    )]);
    let err = client(&stub.base_url, "lk_revoked")
        .check("llmstxt.org")
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::InvalidKey { detail } if detail == "Unknown API key."),
        "{err:?}"
    );
    assert_eq!(err.status(), Some(401));
    assert_eq!(err.code(), Some("invalid_key"));
}

#[tokio::test]
async fn rate_limited_with_retry_after_seconds() {
    let stub = serve(vec![problem(
        "429 Too Many Requests",
        "retry-after: 30\r\n",
        r#"{"title":"Too Many Requests","status":429,"code":"rate_limited","detail":"Slow down."}"#,
    )]);
    let err = client(&stub.base_url, "")
        .llms("llmstxt.org", LlmsOptions::default())
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::RateLimited { retry_after: Some(d), .. } if d == Duration::from_secs(30)),
        "{err:?}"
    );
    assert_eq!(err.to_string(), "Slow down. (retry in 30s)");
}

#[tokio::test]
async fn quota_exceeded_with_retry_after_http_date() {
    let later = httpdate::fmt_http_date(std::time::SystemTime::now() + Duration::from_secs(3600));
    let stub = serve(vec![problem(
        "429 Too Many Requests",
        &format!("retry-after: {later}\r\n"),
        r#"{"title":"Too Many Requests","status":429,"code":"quota_exceeded","detail":"Daily quota used up."}"#,
    )]);
    let err = client(&stub.base_url, "lk_test")
        .llms("example.com", LlmsOptions::default())
        .await
        .unwrap_err();
    let Error::QuotaExceeded {
        retry_after: Some(wait),
        ..
    } = err
    else {
        panic!("{err:?}")
    };
    assert!(
        wait > Duration::from_secs(3500) && wait <= Duration::from_secs(3600),
        "{wait:?}"
    );
}

#[tokio::test]
async fn other_problems_keep_status_code_and_issues() {
    let stub = serve(vec![problem(
        "400 Bad Request",
        "",
        r#"{"title":"Bad Request","status":400,"code":"invalid_request","detail":"Fix the parameters.","errors":[{"path":"url","message":"Paste a link first."}]}"#,
    )]);
    let err = client(&stub.base_url, "")
        .llms("", LlmsOptions::default())
        .await
        .unwrap_err();
    let Error::Api {
        status,
        code,
        title,
        detail,
        errors,
        retry_after,
    } = err
    else {
        panic!("{err:?}")
    };
    assert_eq!(
        (status, code.as_deref(), title.as_str(), detail.as_str()),
        (
            400,
            Some("invalid_request"),
            "Bad Request",
            "Fix the parameters."
        )
    );
    assert_eq!(errors[0].path, "url");
    assert_eq!(retry_after, None);
}

#[tokio::test]
async fn non_problem_errors_fall_back_to_the_status() {
    let stub = serve(vec![response(
        "502 Bad Gateway",
        "text/html",
        "",
        "<h1>bad gateway</h1>",
    )]);
    let err = client(&stub.base_url, "")
        .check("llmstxt.org")
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::Api { status: 502, code: None, title, detail, .. } if title == "Bad Gateway" && detail == "HTTP 502 Bad Gateway"),
        "{err:?}"
    );
}

#[tokio::test]
async fn unexpected_json_is_a_decode_error() {
    let stub = serve(vec![json(r#"{"hello":"world"}"#)]);
    let err = client(&stub.base_url, "")
        .check("llmstxt.org")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Decode(_)), "{err:?}");
}

#[tokio::test]
async fn malformed_key_fails_without_sending() {
    let client = client("http://127.0.0.1:9", "lk_bad\nkey");
    assert!(client.has_key());
    assert!(!format!("{client:?}").contains("lk_bad"));
    let err = client
        .llms("llmstxt.org", LlmsOptions::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidKey { .. }), "{err:?}");
}

#[tokio::test]
async fn unreachable_is_an_http_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let err = client(&base_url, "")
        .check("llmstxt.org")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Http(_)), "{err:?}");
    assert!(
        err.to_string().starts_with("couldn't reach llms.ink"),
        "{err}"
    );
}

#[test]
fn debug_hides_the_key() {
    let client = Client::builder()
        .base_url("https://example.test")
        .key("lk_secret")
        .build()
        .unwrap();
    let debug = format!("{client:?}");
    assert!(!debug.contains("lk_secret"), "{debug}");
    assert!(debug.contains("key: \"set\""), "{debug}");
    assert_eq!(client.base_url(), "https://example.test");
}
