//! The blocking client: llms-full.txt for a link.
//!
//! ```sh
//! cargo run --example blocking --features blocking -- docs.astro.build/en/guides/
//! ```

use llms_ink::blocking::Client;
use llms_ink::{Error, LlmsOptions, Mode};

fn main() -> Result<(), Error> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "llmstxt.org".into());
    let options = LlmsOptions {
        mode: Mode::Full,
        fresh: false,
    };
    let full = Client::new().llms(&url, options)?;
    println!(
        "{} ({} pages, {} tokens)",
        full.title, full.pages, full.tokens
    );
    println!("{}", full.text);
    Ok(())
}
