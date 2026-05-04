use super::CitationParser;
use regex::Regex;
use std::collections::HashSet;

pub struct TypstParser {
    // Matches #cite(<key>) with optional arguments
    cite_fn_regex: Regex,
    // Matches @key shorthand syntax
    at_regex: Regex,
}

impl TypstParser {
    pub fn new() -> Self {
        let cite_fn_regex = Regex::new(r"#cite\(<([^>]+)>(?:,[^)]*)?\.?\)").unwrap();
        // @key: starts with @, followed by alphanumeric, colons, dots, hyphens, underscores
        // Must not be preceded by alphanumeric (to avoid matching emails etc.)
        let at_regex = Regex::new(r"(?:^|[^a-zA-Z0-9])@([a-zA-Z0-9][a-zA-Z0-9:_\-]*)").unwrap();

        Self {
            cite_fn_regex,
            at_regex,
        }
    }
}

impl Default for TypstParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CitationParser for TypstParser {
    fn extract_citations(&self, content: &str) -> Vec<String> {
        let mut citations = HashSet::new();

        // Match #cite(<key>)
        for cap in self.cite_fn_regex.captures_iter(content) {
            if let Some(key) = cap.get(1) {
                let key = key.as_str().trim();
                if !key.is_empty() {
                    citations.insert(key.to_string());
                }
            }
        }

        // Match @key
        for cap in self.at_regex.captures_iter(content) {
            if let Some(key) = cap.get(1) {
                let key = key.as_str().trim();
                if !key.is_empty() {
                    citations.insert(key.to_string());
                }
            }
        }

        let mut result: Vec<_> = citations.into_iter().collect();
        result.sort();
        result
    }

    fn to_minimal_tex(&self, keys: &[String]) -> String {
        // Convert Typst citations to minimal TeX format for the API
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
        let parser = TypstParser::new();
        let content = r"#cite(<Einstein1905>)";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Einstein1905"]);
    }

    #[test]
    fn test_cite_with_form() {
        let parser = TypstParser::new();
        let content = r#"#cite(<Feynman1948>, form: "prose")"#;
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Feynman1948"]);
    }

    #[test]
    fn test_multiple_citations() {
        let parser = TypstParser::new();
        let content = r"#cite(<Key1>) and #cite(<Key2>) and #cite(<Key3>)";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Key1", "Key2", "Key3"]);
    }

    #[test]
    fn test_deduplicate() {
        let parser = TypstParser::new();
        let content = r"#cite(<Same>) and #cite(<Same>)";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Same"]);
    }

    #[test]
    fn test_to_minimal_tex() {
        let parser = TypstParser::new();
        let keys = vec!["Key1".to_string(), "Key2".to_string()];
        let tex = parser.to_minimal_tex(&keys);
        assert!(tex.contains(r"\cite{Key1}"));
        assert!(tex.contains(r"\cite{Key2}"));
    }

    #[test]
    fn test_arxiv_style_key() {
        let parser = TypstParser::new();
        let content = r"#cite(<1706.03762>)";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["1706.03762"]);
    }

    #[test]
    fn test_at_syntax() {
        let parser = TypstParser::new();
        let content = "as shown by @CMS:2022dwd and @ATLAS:2022vkf.";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["ATLAS:2022vkf", "CMS:2022dwd"]);
    }

    #[test]
    fn test_at_syntax_start_of_line() {
        let parser = TypstParser::new();
        let content = "@Weinberg:1967tq proved that...";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Weinberg:1967tq"]);
    }

    #[test]
    fn test_mixed_cite_and_at() {
        let parser = TypstParser::new();
        let content = "#cite(<Key1>) and @Key2";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Key1", "Key2"]);
    }

    #[test]
    fn test_at_dedup_with_cite() {
        let parser = TypstParser::new();
        let content = "#cite(<Same>) and @Same";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Same"]);
    }
}
