use clap::Parser;
use inspire_bibgen::api::{BibliographyGenerator, InspireClient};
use inspire_bibgen::identifier::Identifier;
use inspire_bibgen::output::save_bibtex;
use std::path::{Path, PathBuf};
use std::process;

/// INSPIRE HEP BibTeX Generator
///
/// Fetch BibTeX entries from INSPIRE HEP database.
/// Automatically detects identifier type (arXiv, DOI, INSPIRE ID, texkey)
/// or processes TeX/Typst files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Identifier(s) or file path. Comma-separated for multiple items.
    ///
    /// Examples:
    ///   1706.03762                          (arXiv)
    ///   10.1103/PhysRevLett.19.1264         (DOI)
    ///   451647                              (INSPIRE ID)
    ///   ATLAS:2025kvb                       (texkey)
    ///   Haisch:2013fla,Buckley:2014fba      (multiple texkeys)
    ///   paper.tex                           (TeX file)
    ///   paper.typ                           (Typst file)
    input: String,

    /// Output file path (if not specified, prints to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Overwrite output file instead of appending
    #[arg(long)]
    overwrite: bool,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(args: Args) -> inspire_bibgen::Result<()> {
    let input = args.input.trim();
    let path = Path::new(input);

    // Treat the input as a document only when it has a recognised extension;
    // otherwise it is one or more identifiers. This avoids an identifier being
    // silently overridden by a same-named file on disk.
    let doc_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .filter(|e| matches!(e.as_str(), "tex" | "typ" | "typst"));

    let bibtex = if let Some(ext) = doc_ext {
        if !path.is_file() {
            return Err(inspire_bibgen::Error::Parse(format!(
                "File not found: {}",
                input
            )));
        }
        let bibgen = BibliographyGenerator::new();
        eprintln!("Processing {} file: {}...", ext, input);
        bibgen.generate_from_file(path)?
    } else {
        // Split by comma for multiple identifiers
        let keys: Vec<&str> = input
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if keys.is_empty() {
            return Err(inspire_bibgen::Error::InvalidIdentifier(
                "no identifier provided".to_string(),
            ));
        }

        let client = InspireClient::new();
        let mut results = Vec::new();
        // Keys that could not be retrieved, paired with the reason.
        let mut missing_keys: Vec<(String, String)> = Vec::new();

        for key in &keys {
            // A malformed identifier or a failed fetch only skips that item —
            // it never aborts the whole batch.
            let id = match Identifier::parse(key) {
                Ok(id) => id,
                Err(e) => {
                    missing_keys.push((key.to_string(), e.to_string()));
                    continue;
                }
            };
            eprintln!("Fetching BibTeX for {:?}...", id);
            match client.fetch_bibtex(&id) {
                Ok(bib) => results.push(bib),
                Err(e) => missing_keys.push((key.to_string(), e.to_string())),
            }
        }

        // Always tell the user explicitly which keys were left out.
        if let Some(summary) = format_missing_summary(&missing_keys) {
            eprintln!("{}", summary);
        }

        if results.is_empty() {
            return Err(inspire_bibgen::Error::Api(
                "No BibTeX entries retrieved".to_string(),
            ));
        }

        results.join("\n\n")
    };

    // Output: print to stdout or save to file
    match args.output {
        Some(output_path) => {
            let append = !args.overwrite;
            save_bibtex(&output_path, &bibtex, append)?;
        }
        None => {
            print!("{}", bibtex);
        }
    }

    Ok(())
}

/// Build an explicit, user-facing summary of the keys that could not be
/// retrieved from INSPIRE. Returns `None` when every key succeeded.
fn format_missing_summary(missing: &[(String, String)]) -> Option<String> {
    if missing.is_empty() {
        return None;
    }
    let mut s = format!(
        "\n⚠  {} key(s) could not be retrieved from INSPIRE and were left out of the output:",
        missing.len()
    );
    for (key, reason) in missing {
        // Keep the summary scannable: only the first line of the reason.
        let reason = reason.lines().next().unwrap_or("unknown error");
        s.push_str(&format!("\n    - {key}  ({reason})"));
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_summary_when_nothing_failed() {
        assert!(format_missing_summary(&[]).is_none());
    }

    #[test]
    fn summary_lists_every_failed_key_with_its_reason() {
        let missing = vec![
            (
                "9999.99999".to_string(),
                "Paper not found: ArXiv(\"9999.99999\")".to_string(),
            ),
            (
                "badkey".to_string(),
                "Invalid identifier format: bad".to_string(),
            ),
        ];
        let summary = format_missing_summary(&missing).expect("summary expected");
        assert!(summary.contains("2 key"), "should state the count: {summary}");
        assert!(summary.contains("9999.99999"));
        assert!(summary.contains("Paper not found"));
        assert!(summary.contains("badkey"));
        assert!(summary.contains("Invalid identifier format"));
    }

    #[test]
    fn summary_keeps_only_the_first_line_of_a_multiline_reason() {
        let missing = vec![("k".to_string(), "short reason\nverbose detail".to_string())];
        let summary = format_missing_summary(&missing).expect("summary expected");
        assert!(summary.contains("short reason"));
        assert!(!summary.contains("verbose detail"));
    }
}
