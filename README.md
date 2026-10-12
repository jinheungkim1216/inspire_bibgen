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

# Just the texkey, for filling in \cite{...} while writing
inspire_bibgen --key 1706.03762                              # Vaswani:2017lxt
inspire_bibgen --key https://arxiv.org/abs/1706.03762        # URLs work too
inspire_bibgen --key 'doi:10.1103/PhysRevLett.19.1264,CMS:2022dwd'
```

## Supported Identifier Formats

| Type | Example |
|------|---------|
| arXiv (new) | `1706.03762`, `2301.12345v2` |
| arXiv (old) | `hep-th/9711200` |
| DOI | `10.1103/PhysRevLett.19.1264` |
| INSPIRE ID | `1421100` |
| Texkey | `ATLAS:2025kvb`, `Weinberg:1967tq` |
| Prefixed | `arXiv:1706.03762`, `doi:10.1103/PhysRevLett.19.1264` |
| URL | `https://arxiv.org/abs/1706.03762`, `https://arxiv.org/pdf/1706.03762v2`, `https://doi.org/10.1103/…`, `https://inspirehep.net/literature/1421100`, publisher pages with the DOI in the path (APS, Springer, …) |

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
      --key              Print only the INSPIRE texkey(s), one per line, instead of BibTeX
  -h, --help             Print help
  -V, --version          Print version
```

## macOS Quick Actions

Two Quick Actions let you turn an arXiv ID, DOI or URL into an INSPIRE texkey
without leaving your editor or browser. They live in `mac/` and only call the
installed `inspire_bibgen` binary.

```bash
cargo install --path .   # the Quick Actions need the binary in ~/.cargo/bin
mac/install.sh           # installs into ~/Library/Services and binds the shortcuts
mac/install.sh --uninstall
```

| Quick Action | Shortcut | What it does |
|---|---|---|
| **INSPIRE texkey** | ⌃⌘K | Replaces the selected text with its texkey(s). Select `1706.03762` inside `\cite{1706.03762}` and press the shortcut: it becomes `\cite{Vaswani:2017lxt}`. Comma-, semicolon- or newline-separated lists work too. |
| **INSPIRE texkey from clipboard** | ⌃⇧⌘K | Looks up whatever is on the clipboard (an arXiv URL copied from Safari, say), puts the texkey back on the clipboard and shows a notification. Works with nothing selected. |

IDs that INSPIRE cannot resolve are left exactly as they were and named in a
notification, so nothing is lost from the document. Both actions also appear
under the app menu › Services and in the right-click menu of selected text.
`install.sh` pre-fills the shortcuts, but macOS does not always honour
shortcuts written this way. If a shortcut does not fire, set it once in
System Settings › Keyboard › Keyboard Shortcuts › Services › Text. Until then
the actions are two clicks away: select the ID, right-click › Services ›
INSPIRE texkey, or use the app menu › Services. ⌃⌘ was chosen because window
managers such as Rectangle take most ⌃⌥+letter combinations before apps see
them.
