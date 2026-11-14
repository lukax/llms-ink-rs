//! The async client and its builder, plus the configuration the blocking client shares.

use std::fmt;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;

use crate::error::{finish, retry_after};
use crate::{CheckResult, Error, LlmsOptions, LlmsResult};

/// The API's origin, used unless [`ClientBuilder::base_url`] or `LLMS_INK_API` says otherwise.
pub const DEFAULT_BASE_URL: &str = "https://llms.ink";

/// How long a request may take by default. Building a big site or a video can take minutes.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

const KEY_ENV: &str = "LLMS_INK_KEY";
const BASE_URL_ENV: &str = "LLMS_INK_API";

/// Builds a [`Client`] (or a `blocking::Client`) with a key, base URL or timeout.
///
/// ```no_run
/// # fn main() -> Result<(), llms_ink::Error> {
/// let client = llms_ink::Client::builder()
///     .key("lk_...")
///     .timeout(std::time::Duration::from_secs(60))
///     .build()?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct ClientBuilder {
    key: Option<String>,
    base_url: Option<String>,
    timeout: Duration,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            key: None,
            base_url: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl ClientBuilder {
    /// The API key (`lk_…`), sent as `Authorization: Bearer`. Defaults to the `LLMS_INK_KEY`
    /// environment variable; an empty key means anonymous, even when the variable is set.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// The API's origin. Defaults to the `LLMS_INK_API` environment variable, then
    /// [`DEFAULT_BASE_URL`].
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// The whole-request timeout. Defaults to [`DEFAULT_TIMEOUT`].
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builds the async client. Fails only when the TLS backend can't be initialized.
    pub fn build(self) -> Result<Client, Error> {
        let config = self.config();
        let http = reqwest::Client::builder()
            .default_headers(config.headers())
            .timeout(config.timeout)
            .build()?;
        Ok(Client { http, config })
    }

    pub(crate) fn config(self) -> Config {
        let key = self.key.or_else(|| std::env::var(KEY_ENV).ok());
        let base_url = self
            .base_url
            .or_else(|| std::env::var(BASE_URL_ENV).ok().filter(|v| !v.is_empty()))
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        Config {
            base_url: base_url.trim_end_matches('/').to_owned(),
            auth: Auth::new(key.as_deref()),
            timeout: self.timeout,
        }
    }
}

/// The settings both clients share.
#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) base_url: String,
    pub(crate) auth: Auth,
    pub(crate) timeout: Duration,
}

impl Config {
    /// Headers sent with every request.
    pub(crate) fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static(crate::USER_AGENT));
        if let Auth::Bearer(value) = &self.auth {
            headers.insert(AUTHORIZATION, value.clone());
        }
        headers
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    pub(crate) fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name)
            .field("base_url", &self.base_url)
            .field("key", &self.auth.describe())
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// The `Authorization` header, decided once when the client is built.
#[derive(Clone)]
pub(crate) enum Auth {
    Anonymous,
    Bearer(HeaderValue),
    /// A key that can't be sent in a header: every request fails with [`Error::InvalidKey`] rather
    /// than going out anonymously.
    Malformed,
}

impl Auth {
    fn new(key: Option<&str>) -> Self {
        match key.map(str::trim) {
            None | Some("") => Auth::Anonymous,
            Some(key) => match HeaderValue::from_str(&format!("Bearer {key}")) {
                Ok(mut value) => {
                    value.set_sensitive(true);
                    Auth::Bearer(value)
                }
                Err(_) => Auth::Malformed,
            },
        }
    }

    /// Fails for a malformed key, before anything is sent.
    pub(crate) fn ready(&self) -> Result<(), Error> {
        match self {
            Auth::Malformed => Err(Error::InvalidKey {
                detail: "The API key has characters that can't be sent in an HTTP header.".into(),
            }),
            _ => Ok(()),
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Auth::Anonymous => "none",
            Auth::Bearer(_) => "set",
            Auth::Malformed => "malformed",
        }
    }
}

/// Query parameters of `GET /api/v0/llms`.
pub(crate) fn llms_query(url: &str, options: LlmsOptions) -> Vec<(&'static str, &str)> {
    let mut query = vec![("url", url), ("mode", options.mode.as_str())];
    if options.fresh {
        query.push(("fresh", "1"));
    }
    query
}

/// An async client for <https://llms.ink>. Needs a Tokio runtime.
///
/// Cloning is cheap and shares the connection pool.
///
/// ```no_run
/// use llms_ink::{Client, LlmsOptions, Mode};
///
/// # async fn run() -> Result<(), llms_ink::Error> {
/// let client = Client::new();
/// let docs = client
///     .llms("docs.astro.build/en/guides/", LlmsOptions { mode: Mode::Full, fresh: false })
///     .await?;
/// println!("{} ({} tokens)\n{}", docs.title, docs.tokens, docs.text);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    config: Config,
}

impl Client {
    /// A client with the key from `LLMS_INK_KEY` (anonymous when unset) and the default base URL.
    ///
    /// # Panics
    ///
    /// When the TLS backend can't be initialized, like [`reqwest::Client::new`]; use
    /// [`Client::builder`] to handle that as an error.
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("llms_ink::Client::new: couldn't initialize the HTTP client")
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
    pub async fn llms(&self, url: &str, options: LlmsOptions) -> Result<LlmsResult, Error> {
        self.get("/api/v0/llms", &llms_query(url, options)).await
    }

    /// Checks a site's published llms.txt against the format and that its links answer.
    pub async fn check(&self, url: &str) -> Result<CheckResult, Error> {
        self.get("/api/v0/check", &[("url", url)]).await
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, Error> {
        self.config.auth.ready()?;
        let response = self
            .http
            .get(self.config.url(path))
            .query(query)
            .send()
            .await?;
        let status = response.status();
        let wait = retry_after(response.headers());
        let body = response.bytes().await?;
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
