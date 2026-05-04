use super::CitationParser;
use regex::Regex;
use std::collections::HashSet;

pub struct TexParser {
    // Matches various citation commands: \cite, \citep, \citet, \citeyear, etc.
    // Handles optional arguments like \cite[note]{key}
    citation_regex: Regex,
}

impl TexParser {
    pub fn new() -> Self {
        // Match citation commands with various prefixes and optional arguments
        // Examples: \cite{key}, \citep{key}, \citet{key}, \cite[p.~5]{key}, \citep*{key}
        let citation_regex =
            Regex::new(r"\\cite[tp]?\*?(?:\[[^\]]*\])*(?:\[[^\]]*\])?\{([^}]+)\}").unwrap();

        Self { citation_regex }
    }
}

impl Default for TexParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CitationParser for TexParser {
    fn extract_citations(&self, content: &str) -> Vec<String> {
        let mut citations = HashSet::new();

        for cap in self.citation_regex.captures_iter(content) {
            if let Some(keys) = cap.get(1) {
                // Split by comma for multiple keys: \cite{key1,key2,key3}
                for key in keys.as_str().split(',') {
                    let key = key.trim();
                    if !key.is_empty() {
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
}
