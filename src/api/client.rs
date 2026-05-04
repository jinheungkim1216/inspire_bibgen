use crate::identifier::Identifier;
use crate::{Error, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;

pub struct InspireClient {
    client: Client,
}

impl InspireClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch BibTeX for a single paper by its identifier
    pub fn fetch_bibtex(&self, identifier: &Identifier) -> Result<String> {
        let url = identifier.api_endpoint();

        let response = self.client.get(&url).send()?;

        match response.status() {
            StatusCode::OK => {
                let bibtex = response.text()?;
                Ok(bibtex)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("{:?}", identifier))),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimited),
            status => Err(Error::Api(format!(
                "Unexpected status code: {} for {}",
                status, url
            ))),
        }
    }
}

impl Default for InspireClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Integration test - requires network
    fn test_fetch_arxiv() {
        let client = InspireClient::new();
        let id = Identifier::ArXiv("1706.03762".to_string());
        let bibtex = client.fetch_bibtex(&id).unwrap();
        assert!(bibtex.contains("@article"));
    }
}
