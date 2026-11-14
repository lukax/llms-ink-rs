# llms-ink

Turn any public link — a site, docs section, page, GitHub repo, PDF or video — into
[llms.txt](https://llmstxt.org) or llms-full.txt, from Rust. A client for [llms.ink](https://llms.ink).

```sh
cargo add llms-ink
cargo add tokio --features macros,rt-multi-thread
```

Needs Rust 1.85 or later.

## Use

```rust
use llms_ink::{Client, LlmsOptions, Mode};

#[tokio::main]
async fn main() -> Result<(), llms_ink::Error> {
    let client = Client::new();

    // llms.txt: an index of links
    let index = client.llms("llmstxt.org", LlmsOptions::default()).await?;

    // llms-full.txt: the content
    let docs = client
        .llms("docs.astro.build/en/guides/", LlmsOptions { mode: Mode::Full, fresh: false })
        .await?;

    println!("{} {:?} {} tokens", docs.title, docs.kind, docs.tokens);
    println!("{}", docs.text);
    Ok(())
}
```

`llms()` returns an `LlmsResult`: `url`, `kind` (`Site`, `Section`, `Page`, `Repo`, `Document`,
`Video`), `source` (`Native` = published by the site, `Generated` = made by llms.ink), `mode`, `title`,
`text`, `tokens`, `pages`, `model`, `cached`, `generated_at`, and `media` for videos (duration,
downloads, transcript segments). `fresh: true` skips the cache and rebuilds.

Check a site's published llms.txt:

```rust
let report = client.check("llmstxt.org").await?;
println!("{} {} errors", report.found, report.summary.errors);
for issue in &report.issues {
    println!("{:?} {}: {}", issue.level, issue.code, issue.message);
}
```

Check llms.txt you haven't published (`POST /api/v0/check`, up to 2 MB):

```rust
use llms_ink::CheckTextOptions;

let text = std::fs::read_to_string("llms.txt")?;
let options = CheckTextOptions { base: Some("https://example.com"), links: true };
let report = client.check_text(text, options).await?;
if let Some(parsed) = &report.parsed {
    for section in &parsed.sections {
        for link in &section.links {
            println!("{} line {}: {} {}", section.name, link.line, link.title, link.url);
        }
    }
}
```

`base` resolves relative links and is reported as `site`/`url` (both `None` without it); `links`
also checks up to 25 linked URLs and, with `base`, llms-full.txt (`llms_full` is `None` when not
checked). Both `check` and `check_text` return `parsed`: `title`, `summary`, `details` and
`sections` with their `links` (`title`, `url`, `notes`, `line`), or `None` when nothing was found.

0.2.0 is a breaking release: `CheckResult::site`/`url` are `Option<String>` and `llms_full` is
`Option<bool>`. See [CHANGELOG.md](CHANGELOG.md).

### Blocking

```toml
llms-ink = { version = "0.2", features = ["blocking"] }
```

```rust
use llms_ink::{blocking::Client, LlmsOptions};

let index = Client::new().llms("llmstxt.org", LlmsOptions::default())?;
```

## API key (optional)

Without a key you share the free anonymous limits (10 requests/minute, 60 new links a day per IP;
cached links are free). A key gets its own daily quota:

```sh
export LLMS_INK_KEY=lk_...
```

or `Client::builder().key("lk_...").build()?`. It is sent as `Authorization: Bearer`. A bad key fails
with `Error::InvalidKey`; it never falls back to anonymous. `.key("")` forces anonymous requests.
[Ask for a key](https://lucasespindola.com/message?text=I%27d+like+an+llms.ink+API+key).

## Errors

```rust
use llms_ink::{Client, Error, LlmsOptions};

match Client::new().llms("example.com", LlmsOptions::default()).await {
    Ok(result) => println!("{}", result.text),
    Err(Error::RateLimited { retry_after, .. } | Error::QuotaExceeded { retry_after, .. }) => {
        println!("wait {retry_after:?}")
    }
    Err(Error::InvalidKey { detail }) => println!("bad key: {detail}"),
    Err(Error::Api { status, code, detail, .. }) => println!("{status} {code:?}: {detail}"),
    Err(err) => println!("{err}"), // Http (couldn't connect) or Decode
}
```

`Error::Api` covers the rest of the [error codes](https://llms.ink/docs/#errors): `invalid_url`,
`unprocessable`, `upstream_error`, `timeout`… `err.status()`, `err.code()` and `err.retry_after()` work
on any error.

## Settings

| Builder | Environment | Default |
| --- | --- | --- |
| `.key(..)` | `LLMS_INK_KEY` | anonymous |
| `.base_url(..)` | `LLMS_INK_API` | `https://llms.ink` |
| `.timeout(..)` | | 300 s (big sites and videos take minutes) |

## Features

- `rustls` (default): TLS with rustls and the platform's certificate store.
- `native-tls`: the system TLS library instead (use `default-features = false`).
- `blocking`: `llms_ink::blocking::Client`.

Requests send `User-Agent: llms-ink-rust/<version>`.

## Also

- Without a client: `https://llms.ink/<url>` returns llms.txt and `https://llms.ink/full/<url>`
  llms-full.txt as plain text.
- MCP server: `https://llms.ink/mcp`.
- [API docs](https://llms.ink/docs/)

## License

MIT
