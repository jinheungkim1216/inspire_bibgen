pub mod tex;
pub mod typst;

pub use tex::TexParser;
pub use typst::TypstParser;

/// Common trait for citation parsers
pub trait CitationParser {
    /// Extract citation keys from the content
    fn extract_citations(&self, content: &str) -> Vec<String>;

    /// Convert citation keys to minimal TeX format for API upload
    fn to_minimal_tex(&self, keys: &[String]) -> String;
}
