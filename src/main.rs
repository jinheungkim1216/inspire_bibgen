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

    // Check if input is a file
    let bibtex = if path.exists() && path.is_file() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext.to_lowercase().as_str() {
            "tex" => {
                let bibgen = BibliographyGenerator::new();
                eprintln!("Processing TeX file: {}...", input);
                bibgen.generate_from_file(path)?
            }
            "typ" | "typst" => {
                let bibgen = BibliographyGenerator::new();
                eprintln!("Processing Typst file: {}...", input);
                bibgen.generate_from_file(path)?
            }
            _ => {
                return Err(inspire_bibgen::Error::Parse(format!(
                    "Unsupported file type: .{}",
                    ext
                )));
            }
        }
    } else {
        // Split by comma for multiple identifiers
        let keys: Vec<&str> = input.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let client = InspireClient::new();
        let mut results = Vec::new();

        for key in &keys {
            let id = Identifier::parse(key)?;
            eprintln!("Fetching BibTeX for {:?}...", id);
            match client.fetch_bibtex(&id) {
                Ok(bib) => results.push(bib),
                Err(e) => eprintln!("Warning: {} - skipping", e),
            }
        }

        if results.is_empty() {
            return Err(inspire_bibgen::Error::Api("No BibTeX entries retrieved".to_string()));
        }

        results.join("\n")
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
