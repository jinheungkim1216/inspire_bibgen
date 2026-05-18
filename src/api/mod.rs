pub mod bibgen;
pub mod client;

pub use bibgen::BibliographyGenerator;
pub use client::InspireClient;

use reqwest::blocking::Client;
use std::time::Duration;

/// Build a blocking HTTP client with a request timeout and a User-Agent.
///
/// The timeout prevents a stalled connection from hanging the CLI forever,
/// and the User-Agent lets INSPIRE identify (and not over-throttle) the tool.
pub(crate) fn build_client() -> Client {
    Client::builder()
        .user_agent(concat!("inspire_bibgen/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("HTTP client builds from a valid static configuration")
}
