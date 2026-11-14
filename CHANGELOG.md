# Changelog

## 0.2.1 — 2026-11-14

- `Kind::Channel`: YouTube channels and playlists (previously decoded as `Kind::Unknown`).

## 0.2.0 — 2026-11-14

- `Client::check_text(text, CheckTextOptions { base, links })` (and on `blocking::Client`) checks
  llms.txt content through `POST /api/v0/check`.
- `CheckResult::parsed`: the file's structure as `Parsed`, `ParsedSection` and `ParsedLink`.
- **Breaking:** `CheckResult::site` and `url` are `Option<String>` (`None` for `check_text`
  without `base`); `llms_full` is `Option<bool>` (`None` when not checked).

## 0.1.0 — 2026-11-14

- `Client::llms(url, LlmsOptions { mode, fresh })` and `Client::check(url)` for the llms.ink JSON API,
  async on reqwest (rustls by default, `native-tls` optional).
- `blocking::Client` behind the `blocking` feature.
- Optional API key from `ClientBuilder::key` or `LLMS_INK_KEY`, sent as `Authorization: Bearer`;
  base URL from `ClientBuilder::base_url` or `LLMS_INK_API`.
- Typed errors from problem details: `InvalidKey`, `RateLimited` and `QuotaExceeded` (with
  `retry_after`), `Api`, `Http`, `Decode`.
- `User-Agent: llms-ink-rust/<version>`. MSRV 1.85.
