use crate::{Error, Result};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use regex::Regex;
use std::sync::LazyLock;

// arXiv new format: YYMM.NNNNN (e.g. 1706.03762, 2301.12345), optional version.
static ARXIV_NEW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}\.\d{4,5}(?:v\d+)?$").unwrap());
// arXiv old format: category[.SUBCAT]/NNNNNNN (e.g. hep-th/9711200, math.AG/0309285).
static ARXIV_OLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z-]+(?:\.[A-Z]{2})?/\d{7}(?:v\d+)?$").unwrap());
// DOI: 10.XXXX/...
static DOI_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^10\.\d{4,}/\S+$").unwrap());
// texkey: Name:YYYYxxx where YYYY is a plausible 19xx/20xx year.
static TEXKEY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9]*:(?:19|20)\d{2}[A-Za-z0-9]+$").unwrap());

/// Characters that must be percent-encoded when interpolating an identifier
/// into a URL. '/' is intentionally NOT encoded so DOI path separators survive
/// (DOIs are encoded segment-by-segment in `api_endpoint`).
const URL_RESERVED: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'^')
    .add(b'|')
    .add(b'\\')
    .add(b'=');

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

        if ARXIV_NEW.is_match(input) || ARXIV_OLD.is_match(input) {
            return Ok(Identifier::ArXiv(input.to_string()));
        }

        if DOI_PATTERN.is_match(input) {
            return Ok(Identifier::Doi(input.to_string()));
        }

        // INSPIRE record ID is purely numeric.
        if !input.is_empty() && input.chars().all(|c| c.is_ascii_digit()) {
            return Ok(Identifier::Inspire(input.to_string()));
        }

        if TEXKEY_PATTERN.is_match(input) {
            return Ok(Identifier::Texkey(input.to_string()));
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

    /// Get the API endpoint for this identifier.
    ///
    /// DOI and texkey values are percent-encoded so reserved characters
    /// (`#`, `?`, `&`, spaces, …) cannot corrupt the request URL.
    pub fn api_endpoint(&self) -> String {
        match self {
            Identifier::ArXiv(id) => {
                format!("https://inspirehep.net/api/arxiv/{}?format=bibtex", id)
            }
            Identifier::Doi(doi) => {
                // Encode each path segment but keep the '/' separators intact.
                let encoded = doi
                    .split('/')
                    .map(|seg| utf8_percent_encode(seg, URL_RESERVED).to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                format!("https://inspirehep.net/api/doi/{}?format=bibtex", encoded)
            }
            Identifier::Inspire(id) => {
                format!("https://inspirehep.net/api/literature/{}?format=bibtex", id)
            }
            Identifier::Texkey(key) => {
                let encoded = utf8_percent_encode(key, URL_RESERVED);
                format!(
                    "https://inspirehep.net/api/literature?q=texkeys:{}&format=bibtex",
                    encoded
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

    #[test]
    fn test_arxiv_old_format_with_subcategory() {
        // Legacy arXiv IDs can carry a dotted, capitalised subcategory.
        let id = Identifier::parse("math.AG/0309285").unwrap();
        assert_eq!(id, Identifier::ArXiv("math.AG/0309285".to_string()));
    }

    #[test]
    fn test_texkey_parsed() {
        let id = Identifier::parse("ATLAS:2025kvb").unwrap();
        assert_eq!(id, Identifier::Texkey("ATLAS:2025kvb".to_string()));
    }

    #[test]
    fn test_texkey_rejects_non_year_suffix() {
        // `http:1234x` is not a real texkey — the year must look like 19xx/20xx.
        assert!(Identifier::parse("http:1234x").is_err());
    }

    #[test]
    fn test_doi_endpoint_encodes_special_characters() {
        let id = Identifier::Doi("10.1234/foo#bar baz&qux".to_string());
        let url = id.api_endpoint();
        assert!(!url.contains('#'), "raw '#' must be encoded: {url}");
        assert!(!url.contains(' '), "raw space must be encoded: {url}");
        assert!(url.contains("%23") && url.contains("%20") && url.contains("%26"));
        // '/' path separators inside the DOI must be preserved.
        assert!(url.contains("/api/doi/10.1234/foo"));
    }

    #[test]
    fn test_plain_doi_endpoint_unchanged() {
        let id = Identifier::Doi("10.1103/PhysRevLett.19.1264".to_string());
        assert_eq!(
            id.api_endpoint(),
            "https://inspirehep.net/api/doi/10.1103/PhysRevLett.19.1264?format=bibtex"
        );
    }

    #[test]
    fn test_texkey_endpoint_encodes_ampersand() {
        let id = Identifier::Texkey("A&B:2020abc".to_string());
        let url = id.api_endpoint();
        assert!(!url.contains("A&B"), "raw '&' must be encoded: {url}");
        assert!(url.contains("%26"));
    }
}
