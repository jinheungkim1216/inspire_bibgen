# inspire_bibgen

INSPIRE HEP BibTeX Generator - CLI tool to fetch BibTeX entries from the [INSPIRE HEP](https://inspirehep.net/) database.

## Features

- **Auto-detection**: Automatically identifies arXiv IDs, DOIs, INSPIRE record IDs, and texkeys
- **Multiple items**: Comma-separated identifiers for batch fetching
- **TeX/Typst parsing**: Extract citations from `.tex` or `.typ` files and generate a `.bib` file via the INSPIRE Bibliography Generator API
- **Append by default**: New entries are appended to existing `.bib` files with duplicate detection
- **stdout friendly**: Prints to stdout when no output file is specified

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Single identifier (auto-detected)
inspire_bibgen 1706.03762                             # arXiv
inspire_bibgen 10.1103/PhysRevLett.19.1264            # DOI
inspire_bibgen 1421100                                # INSPIRE record ID
inspire_bibgen CMS:2022dwd                            # texkey

# Multiple identifiers (comma-separated)
inspire_bibgen 'CMS:2022dwd,ATLAS:2022vkf'

# Save to file (appends by default)
inspire_bibgen 1706.03762 -o refs.bib
inspire_bibgen CMS:2022dwd,ATLAS:2022vkf -o refs.bib
inspire_bibgen 1706.03762 -o refs.bib --overwrite

# From TeX/Typst files
inspire_bibgen paper.tex -o refs.bib
inspire_bibgen paper.typ -o refs.bib
```

## Supported Identifier Formats

| Type | Example |
|------|---------|
| arXiv (new) | `1706.03762`, `2301.12345v2` |
| arXiv (old) | `hep-th/9711200` |
| DOI | `10.1103/PhysRevLett.19.1264` |
| INSPIRE ID | `1421100` |
| Texkey | `ATLAS:2025kvb`, `Weinberg:1967tq` |

## Supported Citation Formats

**TeX**: `\cite{key}`, `\citep{key}`, `\citet{key}`, `\cite{key1,key2}`, `\cite[note]{key}`

**Typst**: `@key`, `#cite(<key>)`, `#cite(<key>, form: "prose")`

## Options

```
Usage: inspire_bibgen [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Identifier(s) or file path

Options:
  -o, --output <OUTPUT>  Output file path (prints to stdout if omitted)
      --overwrite        Overwrite output file instead of appending
  -h, --help             Print help
  -V, --version          Print version
```
