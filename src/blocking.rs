//! A synchronous client, behind the `blocking` feature.
//!
//! Same API as [`crate::Client`] without `async`. Don't use it inside an async runtime; it runs its
//! own.
//!
//! ```no_run
//! use llms_ink::{blocking::Client, LlmsOptions};
//!
//! # fn main() -> Result<(), llms_ink::Error> {
//! let index = Client::new().llms("llmstxt.org", LlmsOptions::default())?;
//! println!("{}", index.text);
//! # Ok(()) }
//! ```

use std::fmt;
use std::time::Duration;

use serde::de::DeserializeOwned;

use reqwest::header::CONTENT_TYPE;

use crate::client::{check_text_query, llms_query, Auth, Config, TEXT_PLAIN};
use crate::error::{finish, retry_after};
use crate::{CheckResult, CheckTextOptions, Error, LlmsOptions, LlmsResult};

/// Builds a blocking [`Client`]; the same settings as [`crate::ClientBuilder`].
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct ClientBuilder(crate::ClientBuilder);

impl ClientBuilder {
    /// The API key (`lk_…`), sent as `Authorization: Bearer`. Defaults to the `LLMS_INK_KEY`
    /// environment variable; an empty key means anonymous, even when the variable is set.
    pub fn key(self, key: impl Into<String>) -> Self {
        Self(self.0.key(key))
    }

    /// The API's origin. Defaults to the `LLMS_INK_API` environment variable, then
    /// [`DEFAULT_BASE_URL`](crate::DEFAULT_BASE_URL).
    pub fn base_url(self, base_url: impl Into<String>) -> Self {
        Self(self.0.base_url(base_url))
    }

    /// The whole-request timeout. Defaults to [`DEFAULT_TIMEOUT`](crate::DEFAULT_TIMEOUT).
    pub fn timeout(self, timeout: Duration) -> Self {
        Self(self.0.timeout(timeout))
    }

    /// Builds the blocking client. Fails only when the TLS backend can't be initialized.
    pub fn build(self) -> Result<Client, Error> {
        let config = self.0.config();
        let http = reqwest::blocking::Client::builder()
            .default_headers(config.headers())
            .timeout(config.timeout)
            .build()?;
        Ok(Client { http, config })
    }
}

/// A blocking client for <https://llms.ink>.
///
/// Cloning is cheap and shares the connection pool.
#[derive(Clone)]
pub struct Client {
    http: reqwest::blocking::Client,
    config: Config,
}

impl Client {
    /// A client with the key from `LLMS_INK_KEY` (anonymous when unset) and the default base URL.
    ///
    /// # Panics
    ///
    /// When the TLS backend can't be initialized, like [`reqwest::blocking::Client::new`]; use
    /// [`Client::builder`] to handle that as an error.
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("llms_ink::blocking::Client::new: couldn't initialize the HTTP client")
    }

    /// A [`ClientBuilder`] to set the key, base URL or timeout.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// The API's origin, without a trailing slash.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Whether requests carry an API key.
    pub fn has_key(&self) -> bool {
        !matches!(self.config.auth, Auth::Anonymous)
    }

    /// llms.txt ([`Mode::Llms`](crate::Mode::Llms), an index of links) or llms-full.txt
    /// ([`Mode::Full`](crate::Mode::Full), the content inlined) for any public link: a site, docs
    /// section, page, GitHub repo, PDF or video. The scheme is optional.
    pub fn llms(&self, url: &str, options: LlmsOptions) -> Result<LlmsResult, Error> {
        self.get("/api/v0/llms", &llms_query(url, options))
    }

    /// Checks a site's published llms.txt against the format and that its links answer.
    pub fn check(&self, url: &str) -> Result<CheckResult, Error> {
        self.get("/api/v0/check", &[("url", url)])
    }

    /// Checks llms.txt content you have (up to 2,000,000 bytes) without publishing it: the format,
    /// its parsed structure and, with [`CheckTextOptions::links`], its links.
    pub fn check_text(
        &self,
        text: impl Into<String>,
        options: CheckTextOptions<'_>,
    ) -> Result<CheckResult, Error> {
        let request = self
            .http
            .post(self.config.url("/api/v0/check"))
            .query(&check_text_query(options))
            .header(CONTENT_TYPE, TEXT_PLAIN)
            .body(text.into());
        self.send(request)
    }

    fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T, Error> {
        self.send(self.http.get(self.config.url(path)).query(query))
    }

    fn send<T: DeserializeOwned>(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<T, Error> {
        self.config.auth.ready()?;
        let response = request.send()?;
        let status = response.status();
        let wait = retry_after(response.headers());
        let body = response.bytes()?;
        finish(status, wait, &body)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.config.fmt("Client", f)
    }
}
