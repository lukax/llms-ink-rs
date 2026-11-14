//! A client for [llms.ink](https://llms.ink): turn any public link — a site, docs section, page,
//! GitHub repo, PDF or video — into [llms.txt](https://llmstxt.org) or llms-full.txt.
//!
//! ```no_run
//! use llms_ink::{CheckTextOptions, Client, LlmsOptions, Mode};
//!
//! # async fn run() -> Result<(), llms_ink::Error> {
//! let client = Client::new(); // key from LLMS_INK_KEY, anonymous when unset
//!
//! let index = client.llms("llmstxt.org", LlmsOptions::default()).await?;
//! let full = client
//!     .llms("docs.astro.build/en/guides/", LlmsOptions { mode: Mode::Full, fresh: false })
//!     .await?;
//! println!("{}: {} pages, {} tokens", full.title, full.pages, full.tokens);
//!
//! let report = client.check("llmstxt.org").await?;
//! println!("found: {}, errors: {}", report.found, report.summary.errors);
//!
//! // llms.txt you haven't published yet
//! let draft = "# Example\n\n> One line.\n\n## Docs\n\n- [Guide](/guide.md): start here\n";
//! let options = CheckTextOptions { base: Some("https://example.com"), links: false };
//! let report = client.check_text(draft, options).await?;
//! for section in report.parsed.iter().flat_map(|p| &p.sections) {
//!     println!("{}: {} links", section.name, section.links.len());
//! }
//! # Ok(()) }
//! ```
//!
//! # API key
//!
//! Optional. Without one, requests share the anonymous limits (10 requests a minute and 60 new links
//! a day per IP; cached links are free). A key (`lk_…`) gets its own daily quota. It comes from
//! [`ClientBuilder::key`] or the `LLMS_INK_KEY` environment variable and is sent as
//! `Authorization: Bearer`. A bad key fails with [`Error::InvalidKey`]; requests never fall back to
//! anonymous.
//!
//! # Errors
//!
//! Error responses are RFC 9457 problem details, mapped to [`Error`]: [`Error::InvalidKey`],
//! [`Error::RateLimited`] and [`Error::QuotaExceeded`] (with `retry_after`), and [`Error::Api`] for
//! the rest. See <https://llms.ink/docs/#errors>.
//!
//! # Features
//!
//! - `rustls` (default): TLS with rustls and the platform's certificate store.
//! - `native-tls`: TLS with the system library instead.
//! - `blocking`: [`blocking::Client`], a synchronous client.
//!
//! The async [`Client`] runs on Tokio, through `reqwest`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "blocking")]
pub mod blocking;
mod client;
mod error;
mod types;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL, DEFAULT_TIMEOUT};
pub use error::Error;
pub use types::{
    BrokenLink, CheckIssue, CheckLinks, CheckResult, CheckSummary, CheckTextOptions, Issue,
    IssueLevel, Kind, LlmsOptions, LlmsResult, Media, Mode, Parsed, ParsedLink, ParsedSection,
    Segment, Source, Transcript,
};

/// The `User-Agent` every request sends: `llms-ink-rust/<version>`.
pub const USER_AGENT: &str = concat!("llms-ink-rust/", env!("CARGO_PKG_VERSION"));
