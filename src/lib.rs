pub mod api;
pub mod identifier;
pub mod output;
pub mod parser;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Paper not found: {0}")]
    NotFound(String),
    #[error("Invalid identifier format: {0}")]
    InvalidIdentifier(String),
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Rate limited, please wait")]
    RateLimited,
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, Error>;
