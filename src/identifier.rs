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
static DOI_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^10\.\d{4,}/\S+$").unwrap());
// texkey: Name:YYYYxxx where YYYY is a plausible 19xx/20xx year.
static TEXKEY_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9]*:(?:19|20)\d{2}[A-Za-z0-9]+$").unwrap());
// A DOI embedded somewhere in a URL path (doi.org, dx.doi.org, publisher pages).
static DOI_IN_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"10\.\d{4,}/\S+").unwrap());

/// Hosts we accept even when the scheme (`https://`) was left off.
const KNOWN_HOSTS: [&str; 3] = ["arxiv.org", "doi.org", "inspirehep.net"];

/// Reduce browser-style input to a bare identifier.
///
/// Handles `arXiv:` / `doi:` prefixes and URLs from arxiv.org, doi.org,
/// inspirehep.net, and publisher pages that embed a DOI in the path.
/// Returns `None` when the input is neither, so the caller treats it as a
/// bare identifier.
fn strip_url_or_prefix(input: &str) -> Option<String> {
    for (prefix, len) in [("arxiv:", 6), ("doi:", 4)] {
        if input.len() > len && input[..len].eq_ignore_ascii_case(prefix) {
            return Some(input[len..].trim().to_string());
        }
    }

    let rest = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .or_else(|| {
            let host = input.split('/').next().unwrap_or("").to_ascii_lowercase();
            let host = host.strip_prefix("www.").unwrap_or(&host);
            KNOWN_HOSTS
                .iter()
                .any(|h| host == *h || host.ends_with(&format!(".{h}")))
                .then_some(input)
        })?;

    // Drop query string and fragment, then split host from path.
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let is_host = |h: &str| host == h || host.ends_with(&format!(".{h}"));

    if is_host("arxiv.org") {
        let id = path
            .strip_prefix("/abs/")
            .or_else(|| path.strip_prefix("/pdf/"))?
            .trim_end_matches('/');
        let id = id.strip_suffix(".pdf").unwrap_or(id);
        return Some(id.to_string());
    }

    if is_host("inspirehep.net") {
        // /literature/<id>, /api/literature/<id>, legacy /record/<id>
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let pos = segments
            .iter()
            .position(|s| *s == "literature" || *s == "record")?;
        return segments.get(pos + 1).map(|s| s.to_string());
    }

    DOI_IN_PATH.find(path).map(|m| m.as_str().to_string())
}

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
        let original = input.trim();
        let stripped = strip_url_or_prefix(original);
        let input = stripped.as_deref().unwrap_or(original);

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
            - arXiv: 1706.03762, hep-th/9711200, arXiv:1706.03762\n  \
            - DOI: 10.1103/PhysRevLett.19.1264, doi:10.1103/...\n  \
            - INSPIRE ID: 451647\n  \
            - texkey: ATLAS:2025kvb\n  \
            - URL: https://arxiv.org/abs/…, https://doi.org/…, https://inspirehep.net/literature/…",
            original
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

    // ----- URLs and prefixed forms, i.e. what a browser address bar gives you -----

    #[test]
    fn test_url_arxiv_abs() {
        let id = Identifier::parse("https://arxiv.org/abs/1706.03762").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762".to_string()));
    }

    #[test]
    fn test_url_arxiv_abs_keeps_version_and_drops_query() {
        let id = Identifier::parse("https://arxiv.org/abs/1706.03762v5?context=cs").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762v5".to_string()));
    }

    #[test]
    fn test_url_arxiv_pdf_strips_pdf_suffix() {
        let id = Identifier::parse("https://arxiv.org/pdf/1706.03762v2.pdf").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762v2".to_string()));
        let id = Identifier::parse("http://arxiv.org/pdf/1706.03762").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762".to_string()));
    }

    #[test]
    fn test_url_arxiv_old_style_id() {
        let id = Identifier::parse("https://arxiv.org/abs/hep-th/9711200").unwrap();
        assert_eq!(id, Identifier::ArXiv("hep-th/9711200".to_string()));
    }

    #[test]
    fn test_url_arxiv_subdomain_and_missing_scheme() {
        let id = Identifier::parse("https://export.arxiv.org/abs/2301.12345").unwrap();
        assert_eq!(id, Identifier::ArXiv("2301.12345".to_string()));
        let id = Identifier::parse("arxiv.org/abs/2301.12345").unwrap();
        assert_eq!(id, Identifier::ArXiv("2301.12345".to_string()));
    }

    #[test]
    fn test_arxiv_prefix() {
        let id = Identifier::parse("arXiv:1706.03762").unwrap();
        assert_eq!(id, Identifier::ArXiv("1706.03762".to_string()));
        let id = Identifier::parse("arxiv:hep-th/9711200").unwrap();
        assert_eq!(id, Identifier::ArXiv("hep-th/9711200".to_string()));
    }

    #[test]
    fn test_url_doi_org() {
        let id = Identifier::parse("https://doi.org/10.1103/PhysRevLett.19.1264").unwrap();
        assert_eq!(
            id,
            Identifier::Doi("10.1103/PhysRevLett.19.1264".to_string())
        );
        let id = Identifier::parse("http://dx.doi.org/10.1103/PhysRevLett.19.1264").unwrap();
        assert_eq!(
            id,
            Identifier::Doi("10.1103/PhysRevLett.19.1264".to_string())
        );
    }

    #[test]
    fn test_doi_prefix() {
        let id = Identifier::parse("doi:10.1103/PhysRevLett.19.1264").unwrap();
        assert_eq!(
            id,
            Identifier::Doi("10.1103/PhysRevLett.19.1264".to_string())
        );
    }

    #[test]
    fn test_url_publisher_page_with_doi_in_path() {
        // APS and Springer embed the DOI in the article URL.
        let id =
            Identifier::parse("https://journals.aps.org/prl/abstract/10.1103/PhysRevLett.19.1264")
                .unwrap();
        assert_eq!(
            id,
            Identifier::Doi("10.1103/PhysRevLett.19.1264".to_string())
        );
        let id =
            Identifier::parse("https://link.springer.com/article/10.1007/JHEP01(2020)001").unwrap();
        assert_eq!(id, Identifier::Doi("10.1007/JHEP01(2020)001".to_string()));
    }

    #[test]
    fn test_url_doi_drops_fragment() {
        let id = Identifier::parse("https://doi.org/10.1103/PhysRevD.100.015001#abstract").unwrap();
        assert_eq!(
            id,
            Identifier::Doi("10.1103/PhysRevD.100.015001".to_string())
        );
    }

    #[test]
    fn test_url_inspire_literature() {
        let id = Identifier::parse("https://inspirehep.net/literature/1421100").unwrap();
        assert_eq!(id, Identifier::Inspire("1421100".to_string()));
        let id = Identifier::parse("https://inspirehep.net/api/literature/1421100").unwrap();
        assert_eq!(id, Identifier::Inspire("1421100".to_string()));
        let id =
            Identifier::parse("https://inspirehep.net/literature/1421100?ui-citation-summary=true")
                .unwrap();
        assert_eq!(id, Identifier::Inspire("1421100".to_string()));
    }

    #[test]
    fn test_url_without_identifier_is_rejected() {
        assert!(Identifier::parse("https://inspirehep.net/literature?q=higgs").is_err());
        assert!(Identifier::parse("https://example.com/").is_err());
    }
}
