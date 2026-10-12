use super::CitationParser;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// Matches any `\...cite...` command — \cite, \citep, \citet, \autocite,
// \parencite, \textcite, \nocite, \citeauthor, \citealp, … — with an optional
// `*` and any number of optional `[...]` arguments.
static CITATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\[A-Za-z]*cite[A-Za-z]*\*?(?:\[[^\]]*\])*\{([^}]+)\}").unwrap());

/// Strip TeX line comments: everything from an unescaped `%` to end of line.
/// An escaped `\%` (odd number of preceding backslashes) is kept literally.
fn strip_tex_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let mut backslashes = 0usize;
        let mut cut = None;
        for (idx, ch) in line.char_indices() {
            match ch {
                '%' if backslashes.is_multiple_of(2) => {
                    cut = Some(idx);
                    break;
                }
                '\\' => backslashes += 1,
                _ => backslashes = 0,
            }
        }
        match cut {
            Some(idx) => {
                out.push_str(&line[..idx]);
                out.push('\n');
            }
            None => out.push_str(line),
        }
    }
    out
}

pub struct TexParser;

impl TexParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TexParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CitationParser for TexParser {
    fn extract_citations(&self, content: &str) -> Vec<String> {
        let stripped = strip_tex_comments(content);
        let mut citations = HashSet::new();

        for cap in CITATION_REGEX.captures_iter(&stripped) {
            if let Some(keys) = cap.get(1) {
                // Split by comma for multiple keys: \cite{key1,key2,key3}
                for key in keys.as_str().split(',') {
                    let key = key.trim();
                    // `*` is the `\nocite{*}` wildcard, not a citation key.
                    if !key.is_empty() && key != "*" {
                        citations.insert(key.to_string());
                    }
                }
            }
        }

        let mut result: Vec<_> = citations.into_iter().collect();
        result.sort();
        result
    }

    fn to_minimal_tex(&self, keys: &[String]) -> String {
        // Create a minimal TeX document with all citations
        let mut tex = String::from(
            r"\documentclass{article}
\begin{document}
",
        );

        for key in keys {
            tex.push_str(&format!("\\cite{{{}}}\n", key));
        }

        tex.push_str(r"\end{document}");
        tex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_cite() {
        let parser = TexParser::new();
        let content = r"\cite{Einstein1905}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Einstein1905"]);
    }

    #[test]
    fn test_multiple_keys() {
        let parser = TexParser::new();
        let content = r"\cite{Einstein1905,Feynman1948,Hawking1974}";
        let citations = parser.extract_citations(content);
        assert_eq!(
            citations,
            vec!["Einstein1905", "Feynman1948", "Hawking1974"]
        );
    }

    #[test]
    fn test_citep_and_citet() {
        let parser = TexParser::new();
        let content = r"\citep{Key1} and \citet{Key2}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Key1", "Key2"]);
    }

    #[test]
    fn test_cite_with_optional_arg() {
        let parser = TexParser::new();
        let content = r"\cite[p.~42]{Reference2023}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Reference2023"]);
    }

    #[test]
    fn test_deduplicate() {
        let parser = TexParser::new();
        let content = r"\cite{Same} and \cite{Same} and \citep{Same}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Same"]);
    }

    #[test]
    fn test_to_minimal_tex() {
        let parser = TexParser::new();
        let keys = vec!["Key1".to_string(), "Key2".to_string()];
        let tex = parser.to_minimal_tex(&keys);
        assert!(tex.contains(r"\cite{Key1}"));
        assert!(tex.contains(r"\cite{Key2}"));
        assert!(tex.contains(r"\documentclass"));
    }

    #[test]
    fn test_ignores_commented_citations() {
        let parser = TexParser::new();
        let content = "% \\cite{Commented}\n\\cite{Real}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Real"]);
    }

    #[test]
    fn test_escaped_percent_is_not_a_comment() {
        let parser = TexParser::new();
        let content = r"98\% coverage \cite{Real}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Real"]);
    }

    #[test]
    fn test_biblatex_and_natbib_commands() {
        let parser = TexParser::new();
        let content = r"\autocite{A} \parencite{B} \textcite{C} \citeauthor{D} \citealp{E}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["A", "B", "C", "D", "E"]);
    }

    #[test]
    fn test_nocite_star_is_ignored() {
        let parser = TexParser::new();
        let content = r"\nocite{*} \cite{Real}";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Real"]);
    }
}
