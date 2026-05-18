use super::CitationParser;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// TODO: consider migrating Typst citation extraction to the `typst-syntax`
// crate (the official Typst parser). The regexes below cover the documented
// forms (`@key`, `#cite(<key>)`) but can mis-handle `@` / `#cite` inside
// comments, strings, or raw blocks, and cannot resolve `#cite(variable)`.
// A real syntax tree would make extraction context-aware. Deferred for now;
// revisit if real-world `.typ` files surface parsing problems the regexes
// cannot handle. Trade-off: heavier, Typst-version-coupled dependency.

// Matches `#cite(<key>)` with optional trailing arguments.
static CITE_FN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#cite\(<([^>]+)>(?:,[^)]*)?\.?\)").unwrap());

// Matches `@key` shorthand. The key may contain dots *between* segments
// (e.g. arXiv IDs like `1706.03762`) but a trailing `.` — a sentence period —
// is excluded because every `.` must be followed by an alphanumeric character.
// The leading boundary avoids matching `@` preceded by alphanumerics (emails).
static AT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^a-zA-Z0-9])@([a-zA-Z0-9][a-zA-Z0-9:_\-]*(?:\.[a-zA-Z0-9][a-zA-Z0-9:_\-]*)*)")
        .unwrap()
});

pub struct TypstParser;

impl TypstParser {
    pub fn new() -> Self {
        Self
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
        for cap in CITE_FN_REGEX.captures_iter(content) {
            if let Some(key) = cap.get(1) {
                let key = key.as_str().trim();
                if !key.is_empty() {
                    citations.insert(key.to_string());
                }
            }
        }

        // Match @key
        for cap in AT_REGEX.captures_iter(content) {
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

    #[test]
    fn test_at_syntax_arxiv_style_key() {
        // arXiv IDs used as Typst labels contain a dot — `@1706.03762`
        // must not be truncated to `1706`.
        let parser = TypstParser::new();
        let content = "as introduced in @1706.03762 the model...";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["1706.03762"]);
    }

    #[test]
    fn test_at_syntax_trailing_period_excluded() {
        // A sentence-ending period after `@key` is not part of the key.
        let parser = TypstParser::new();
        let content = "shown by @Weinberg:1967tq.";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["Weinberg:1967tq"]);
    }

    #[test]
    fn test_at_reference_directly_after_a_label() {
        // A Typst label `<...>` is a definition, not a citation. An `@key`
        // citation must still be recognised when it directly follows a label
        // (`>` is a valid left boundary), and the bare label is not extracted.
        let parser = TypstParser::new();
        let content = "= Intro <sec:intro>\n\nsee <fig:a>@CMS:2022dwd here";
        let citations = parser.extract_citations(content);
        assert_eq!(citations, vec!["CMS:2022dwd"]);
    }

    #[test]
    fn test_bare_label_is_not_treated_as_citation() {
        // `<key>` on its own (outside `#cite(...)`) is a label definition and
        // must NOT be collected as a citation key.
        let parser = TypstParser::new();
        let content = "#figure(image(\"a.png\")) <plot>";
        let citations = parser.extract_citations(content);
        assert!(citations.is_empty());
    }
}
