use crate::{Error, Result};
use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum Identifier {
    ArXiv(String),
    Doi(String),
    Inspire(String),
    Texkey(String),
}

impl Identifier {
    /// Parse a string into an Identifier, auto-detecting the type
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();

        // Check for arXiv ID patterns
        // New format: YYMM.NNNNN (e.g., 1706.03762, 2301.12345)
        // Old format: category/YYMMNNN (e.g., hep-th/9711200)
        let arxiv_new = Regex::new(r"^(\d{4}\.\d{4,5})(v\d+)?$").unwrap();
        let arxiv_old = Regex::new(r"^[a-z-]+/\d{7}(v\d+)?$").unwrap();

        if arxiv_new.is_match(input) || arxiv_old.is_match(input) {
            return Ok(Identifier::ArXiv(input.to_string()));
        }

        // Check for DOI pattern (10.XXXX/...)
        let doi_pattern = Regex::new(r"^10\.\d{4,}/[^\s]+$").unwrap();
        if doi_pattern.is_match(input) {
            return Ok(Identifier::Doi(input.to_string()));
        }

        // Check for INSPIRE ID (purely numeric)
        if input.chars().all(|c| c.is_ascii_digit()) && !input.is_empty() {
            return Ok(Identifier::Inspire(input.to_string()));
        }

        // Check for texkey pattern (e.g., ATLAS:2025kvb, Weinberg:1967tq)
        // Format: Name:YYYYxxx (flexible suffix)
        if input.contains(':') {
            let texkey_pattern = Regex::new(r"^[A-Za-z][A-Za-z0-9]*:\d{4}[a-zA-Z0-9]+$").unwrap();
            if texkey_pattern.is_match(input) {
                return Ok(Identifier::Texkey(input.to_string()));
            }
        }

        Err(Error::InvalidIdentifier(format!(
            "Cannot determine identifier type for: '{}'\n\
            Supported formats:\n  \
            - arXiv: 1706.03762, hep-th/9711200\n  \
            - DOI: 10.1103/PhysRevLett.19.1264\n  \
            - INSPIRE ID: 451647\n  \
            - texkey: ATLAS:2025kvb",
            input
        )))
    }

    /// Get the API endpoint for this identifier
    pub fn api_endpoint(&self) -> String {
        match self {
            Identifier::ArXiv(id) => {
                format!("https://inspirehep.net/api/arxiv/{}?format=bibtex", id)
            }
            Identifier::Doi(doi) => {
                format!("https://inspirehep.net/api/doi/{}?format=bibtex", doi)
            }
            Identifier::Inspire(id) => {
                format!("https://inspirehep.net/api/literature/{}?format=bibtex", id)
            }
            Identifier::Texkey(key) => {
                format!(
                    "https://inspirehep.net/api/literature?q=texkeys:{}&format=bibtex",
                    key
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arxiv_new_format() {
        let id = Identifier::parse("1706.03762").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762".to_string()));
    }

    #[test]
    fn test_arxiv_new_format_with_version() {
        let id = Identifier::parse("1706.03762v2").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762v2".to_string()));
    }

    #[test]
    fn test_arxiv_old_format() {
        let id = Identifier::parse("hep-th/9711200").unwrap();
        assert_eq!(id, Identifier::ArXiv("hep-th/9711200".to_string()));
    }

    #[test]
    fn test_doi() {
        let id = Identifier::parse("10.1103/PhysRevLett.19.1264").unwrap();
        assert_eq!(
            id,
            Identifier::Doi("10.1103/PhysRevLett.19.1264".to_string())
        );
    }

    #[test]
    fn test_inspire_id() {
        let id = Identifier::parse("451647").unwrap();
        assert_eq!(id, Identifier::Inspire("451647".to_string()));
    }

    #[test]
    fn test_invalid_identifier() {
        let result = Identifier::parse("invalid-id");
        assert!(result.is_err());
    }
}
