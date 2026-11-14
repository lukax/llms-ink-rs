//! Typed errors from the API's RFC 9457 problem details (codes at <https://llms.ink/docs/#errors>).

use std::fmt;
use std::time::{Duration, SystemTime};

use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::Issue;

/// Everything a request can fail with.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// 401 `invalid_key`: the API key is malformed, unknown or revoked. Requests never fall back to
    /// anonymous; drop the key to use the anonymous limits.
    InvalidKey {
        /// What went wrong, for people.
        detail: String,
    },
    /// 429 `rate_limited`: too many requests this minute.
    RateLimited {
        /// How long to wait, from `Retry-After`.
        retry_after: Option<Duration>,
        /// What went wrong, for people.
        detail: String,
    },
    /// 429 `quota_exceeded`: the daily quota of new links is used up. Cached links still work.
    QuotaExceeded {
        /// Time until the quota resets (midnight UTC), from `Retry-After`.
        retry_after: Option<Duration>,
        /// What went wrong, for people.
        detail: String,
    },
    /// Any other error response: `invalid_url`, `unprocessable`, `upstream_error`, `timeout`…
    Api {
        /// The HTTP status code.
        status: u16,
        /// What went wrong, for programs; `None` when the response wasn't problem details.
        code: Option<String>,
        /// The HTTP status title, e.g. `Bad Gateway`.
        title: String,
        /// What went wrong, for people.
        detail: String,
        /// `invalid_request` only: each invalid parameter.
        errors: Vec<Issue>,
        /// `Retry-After`, when the response had one.
        retry_after: Option<Duration>,
    },
    /// The request couldn't be sent or the response couldn't be read: DNS, TLS, connection, timeout,
    /// or an invalid base URL.
    Http(reqwest::Error),
    /// A success response whose body wasn't the expected JSON.
    Decode(serde_json::Error),
}

impl Error {
    /// The HTTP status of an error response; `None` for [`Error::Http`] and [`Error::Decode`].
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::InvalidKey { .. } => Some(401),
            Error::RateLimited { .. } | Error::QuotaExceeded { .. } => Some(429),
            Error::Api { status, .. } => Some(*status),
            Error::Http(err) => err.status().map(|s| s.as_u16()),
            Error::Decode(_) => None,
        }
    }

    /// The problem `code` of an error response, e.g. `invalid_key` or `unprocessable`.
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::InvalidKey { .. } => Some("invalid_key"),
            Error::RateLimited { .. } => Some("rate_limited"),
            Error::QuotaExceeded { .. } => Some("quota_exceeded"),
            Error::Api { code, .. } => code.as_deref(),
            Error::Http(_) | Error::Decode(_) => None,
        }
    }

    /// How long to wait before retrying, when the response said (`Retry-After`).
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimited { retry_after, .. }
            | Error::QuotaExceeded { retry_after, .. }
            | Error::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidKey { detail } => write!(f, "invalid API key: {detail}"),
            Error::RateLimited {
                retry_after,
                detail,
            }
            | Error::QuotaExceeded {
                retry_after,
                detail,
            } => {
                f.write_str(detail)?;
                match retry_after {
                    Some(wait) => write!(f, " (retry in {}s)", wait.as_secs()),
                    None => Ok(()),
                }
            }
            Error::Api {
                status,
                code,
                title,
                detail,
                ..
            } => match code {
                Some(code) => write!(f, "{status} {code}: {detail}"),
                None => write!(f, "{status} {title}: {detail}"),
            },
            Error::Http(err) => write!(f, "couldn't reach llms.ink: {err}"),
            Error::Decode(err) => write!(f, "llms.ink answered with unexpected JSON: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Http(err) => Some(err),
            Error::Decode(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err)
    }
}

/// The parts of a problem body we read; anything missing or malformed falls back to the HTTP status.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Problem {
    title: Option<String>,
    status: Option<u16>,
    code: Option<String>,
    detail: Option<String>,
    errors: Vec<Issue>,
}

/// A response → the decoded body, or the typed error for its problem `code`.
pub(crate) fn finish<T: DeserializeOwned>(
    status: StatusCode,
    retry_after: Option<Duration>,
    body: &[u8],
) -> Result<T, Error> {
    if status.is_success() {
        return serde_json::from_slice(body).map_err(Error::Decode);
    }
    let problem: Problem = serde_json::from_slice(body).unwrap_or_default();
    let title = problem
        .title
        .unwrap_or_else(|| status.canonical_reason().unwrap_or("Error").to_owned());
    let detail = problem
        .detail
        .unwrap_or_else(|| format!("HTTP {} {title}", status.as_u16()));
    Err(match problem.code.as_deref() {
        Some("invalid_key") => Error::InvalidKey { detail },
        Some("rate_limited") => Error::RateLimited {
            retry_after,
            detail,
        },
        Some("quota_exceeded") => Error::QuotaExceeded {
            retry_after,
            detail,
        },
        _ => Error::Api {
            status: problem.status.unwrap_or(status.as_u16()),
            code: problem.code,
            title,
            detail,
            errors: problem.errors,
            retry_after,
        },
    })
}

/// `Retry-After` as a duration: delta-seconds, or an HTTP date turned into the time from now.
pub(crate) fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?.trim();
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let when = httpdate::parse_http_date(value).ok()?;
    Some(
        when.duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO),
    )
}
