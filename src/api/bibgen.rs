use crate::parser::{CitationParser, TexParser, TypstParser};
use crate::{Error, Result};
use reqwest::blocking::{multipart, Client};
use reqwest::StatusCode;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct BibGenResponse {
    data: BibGenData,
}

#[derive(Debug, Deserialize)]
struct BibGenData {
    download_url: String,
    #[serde(default)]
    errors: Vec<BibGenError>,
}

#[derive(Debug, Deserialize)]
struct BibGenError {
    #[allow(dead_code)]
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "ref")]
    reference: Option<String>,
}

pub struct BibliographyGenerator {
    client: Client,
}

impl BibliographyGenerator {
    pub fn new() -> Self {
        Self {
            client: crate::api::build_client(),
        }
    }

    /// Generate bibliography from a TeX or Typst file
    pub fn generate_from_file(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Choose parser based on file extension
        let tex_content = match extension.as_str() {
            "tex" => {
                // For TeX files, extract citations and create minimal TeX
                let parser = TexParser::new();
                let citations = parser.extract_citations(&content);
                if citations.is_empty() {
                    return Err(Error::Parse("No citations found in TeX file".to_string()));
                }
                parser.to_minimal_tex(&citations)
            }
            "typ" | "typst" => {
                // For Typst files, convert to TeX format
                let parser = TypstParser::new();
                let citations = parser.extract_citations(&content);
                if citations.is_empty() {
                    return Err(Error::Parse("No citations found in Typst file".to_string()));
                }
                parser.to_minimal_tex(&citations)
            }
            _ => {
                return Err(Error::Parse(format!(
                    "Unsupported file extension: {}",
                    extension
                )));
            }
        };

        self.upload_tex_content(&tex_content)
    }

    /// Upload TeX content to the bibliography generator API
    fn upload_tex_content(&self, tex_content: &str) -> Result<String> {
        let url = "https://inspirehep.net/api/bibliography-generator";

        // Create multipart form with the TeX content as a file
        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(tex_content.as_bytes().to_vec())
                .file_name("document.tex")
                .mime_str("text/x-tex")
                .unwrap(),
        );

        let response = self
            .client
            .post(url)
            .query(&[("format", "bibtex")])
            .multipart(form)
            .send()?;

        match response.status() {
            StatusCode::OK => {
                // Read the body as text first so a malformed/unexpected
                // response yields a useful error instead of an opaque
                // deserialization failure.
                let body = response.text()?;
                let json: BibGenResponse = serde_json::from_str(&body).map_err(|e| {
                    let snippet: String = body.chars().take(300).collect();
                    Error::Api(format!(
                        "Unexpected response from bibliography generator ({e}): {snippet}"
                    ))
                })?;

                // Report errors if any (but continue)
                for err in &json.data.errors {
                    eprintln!(
                        "Warning: {}{}",
                        err.message,
                        err.reference
                            .as_ref()
                            .map(|r| format!(" (ref: {})", r))
                            .unwrap_or_default()
                    );
                }

                // Download the generated .bib file
                self.download_bib(&json.data.download_url)
            }
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimited),
            status => {
                let body = response.text().unwrap_or_default();
                Err(Error::Api(format!(
                    "Bibliography generator failed with status {}: {}",
                    status, body
                )))
            }
        }
    }

    /// Download the generated .bib file from the given URL
    fn download_bib(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send()?;

        match response.status() {
            StatusCode::OK => Ok(response.text()?),
            status => Err(Error::Api(format!(
                "Failed to download .bib file: status {}",
                status
            ))),
        }
    }
}

impl Default for BibliographyGenerator {
    fn default() -> Self {
        Self::new()
    }
}
