//! Its own test binary: it sets environment variables, which no other test may read concurrently.

mod common;

use common::{json, serve, CHECK};
use llms_ink::{Client, DEFAULT_BASE_URL};

#[tokio::test]
async fn key_and_base_url_come_from_the_environment() {
    std::env::remove_var("LLMS_INK_API");
    std::env::remove_var("LLMS_INK_KEY");
    let client = Client::new();
    assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    assert!(!client.has_key());

    let stub = serve(vec![json(CHECK)]);
    std::env::set_var("LLMS_INK_API", &stub.base_url);
    std::env::set_var("LLMS_INK_KEY", " lk_from_env ");
    let client = Client::new();
    assert!(client.has_key());
    client.check("llmstxt.org").await.unwrap();
    assert!(stub
        .request()
        .contains("authorization: bearer lk_from_env\r\n"));

    // An explicit empty key is anonymous even with LLMS_INK_KEY set.
    assert!(!Client::builder().key("").build().unwrap().has_key());
}
