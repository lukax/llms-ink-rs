//! The API's request options and response bodies, mirroring the JSON schemas at
//! <https://llms.ink/openapi.json>.

use serde::{Deserialize, Serialize};

/// Which file to build for a link.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// `llms.txt`: an index of the link's pages, one line each.
    #[default]
    Llms,
    /// `llms-full.txt`: the content of the pages inlined (up to 20 linked pages).
    Full,
}

impl Mode {
    /// The value the API takes: `llms` or `full`.
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Llms => "llms",
            Mode::Full => "full",
        }
    }
}

/// Options for [`Client::llms`](crate::Client::llms).
///
/// `LlmsOptions::default()` asks for the cached llms.txt index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LlmsOptions {
    /// [`Mode::Llms`] for the index, [`Mode::Full`] for llms-full.txt.
    pub mode: Mode,
    /// Skip the cache and rebuild. Counts against the daily quota of new links.
    pub fresh: bool,
}

/// What a link is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Kind {
    /// A whole site.
    Site,
    /// A docs section: a path and the pages under it.
    Section,
    /// A single page.
    Page,
    /// A GitHub repository.
    Repo,
    /// A file such as a PDF.
    Document,
    /// A video.
    Video,
    /// A kind this version of the crate doesn't know yet.
    #[serde(other)]
    Unknown,
}

/// Where the file came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Source {
    /// Published by the site itself.
    Native,
    /// Made by llms.ink.
    Generated,
    /// A source this version of the crate doesn't know yet.
    #[serde(other)]
    Unknown,
}

/// `GET /api/v0/llms`: the llms.txt or llms-full.txt of a link and what it was built from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmsResult {
    /// The link after normalization.
    pub url: String,
    /// What the link is.
    pub kind: Kind,
    /// Published by the site, or made by llms.ink.
    pub source: Source,
    /// Which file this is.
    pub mode: Mode,
    /// The title of the site, page, repo, document or video.
    pub title: String,
    /// The llms.txt or llms-full.txt content (Markdown).
    pub text: String,
    /// Approximate token count (characters ÷ 4).
    pub tokens: u64,
    /// Links in llms.txt ([`Mode::Llms`]) or pages inlined ([`Mode::Full`]).
    pub pages: u64,
    /// Model that wrote the descriptions; `None` for native files or path-based grouping.
    pub model: Option<String>,
    /// Whether the result came from the cache.
    pub cached: bool,
    /// When the file was built (RFC 3339).
    pub generated_at: String,
    /// Video results only: what `/api/v0/frame` and `/api/v0/media` can serve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<Media>,
}

/// Video results only: what `/api/v0/frame` and `/api/v0/media` can serve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    /// Length in seconds, when known.
    pub duration: Option<f64>,
    /// Whether there is a picture.
    pub has_video: bool,
    /// Whether there is sound.
    pub has_audio: bool,
    /// Thumbnail image URL.
    pub thumbnail: Option<String>,
    /// Downloadable formats: `mp4` and/or `m4a`.
    pub downloads: Vec<String>,
    /// Whether the video is a live stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_live: Option<bool>,
    /// The transcript, with source-video timestamps for selection and clipping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Transcript>,
}

/// A video's transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    /// Where the transcript came from (captions, speech recognition…).
    pub source: String,
    /// The transcript lines, in order.
    pub segments: Vec<Segment>,
}

/// A transcript line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    /// Seconds into the source video where the line starts.
    pub start: f64,
    /// Seconds into the source video where the line ends.
    pub end: f64,
    /// What is said.
    pub text: String,
}

/// `GET /api/v0/check`: a published llms.txt checked against the format and its links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    /// The site's origin.
    pub site: String,
    /// The llms.txt that was checked (or the last place looked when none was found).
    pub url: String,
    /// Whether an llms.txt was found.
    pub found: bool,
    /// HTTP status of the llms.txt request.
    pub status: u16,
    /// The file's H1 title.
    pub title: Option<String>,
    /// Approximate token count (characters ÷ 4).
    pub tokens: u64,
    /// H2 sections.
    pub sections: u64,
    /// The links in the file and which of them are broken.
    pub links: CheckLinks,
    /// Whether llms-full.txt is published next to it.
    pub llms_full: bool,
    /// Errors first, then warnings and notes, in line order.
    pub issues: Vec<CheckIssue>,
    /// Counts of errors and warnings.
    pub summary: CheckSummary,
    /// When the check ran (RFC 3339).
    pub checked_at: String,
}

/// The links of a checked llms.txt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckLinks {
    /// Links in the file.
    pub total: u64,
    /// Links fetched this run (the first 25).
    pub checked: u64,
    /// Links that didn't answer with a success.
    pub broken: Vec<BrokenLink>,
}

/// A link in llms.txt that didn't answer with a success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokenLink {
    /// The link.
    pub url: String,
    /// HTTP status; 0 when it didn't answer in time.
    pub status: u16,
}

/// Severity of a [`CheckIssue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum IssueLevel {
    /// Breaks the format or a link.
    Error,
    /// Worth fixing.
    Warning,
    /// A note.
    Info,
    /// A level this version of the crate doesn't know yet.
    #[serde(other)]
    Unknown,
}

/// A problem found in a checked llms.txt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckIssue {
    /// How serious it is.
    pub level: IssueLevel,
    /// Stable name of the check, e.g. `missing_title`, `broken_link`.
    pub code: String,
    /// Line in the file, when the issue has one.
    pub line: Option<u64>,
    /// What is wrong, for people.
    pub message: String,
}

/// Error and warning counts of a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSummary {
    /// Issues at [`IssueLevel::Error`].
    pub errors: u64,
    /// Issues at [`IssueLevel::Warning`].
    pub warnings: u64,
}

/// An invalid parameter of an `invalid_request` error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// The parameter, e.g. `mode`; nested fields are dotted.
    pub path: String,
    /// What is wrong with it.
    pub message: String,
}
