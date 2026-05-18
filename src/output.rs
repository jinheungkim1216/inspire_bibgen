use crate::Result;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Split BibTeX content into individual entries by brace counting.
///
/// Unlike a naive regex this correctly handles multi-line entries, compact
/// entries whose closing `}` shares a line with the last field, entries
/// without a trailing newline, and nested braces inside field values.
pub fn split_bibtex_entries(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let mut entries = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }
        let start = i;

        // Find this entry's opening brace (stop early if another '@' appears).
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b'{' && bytes[j] != b'@' {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'{' {
            i += 1;
            continue;
        }

        // Walk braces until balanced.
        let mut depth = 0i32;
        let mut k = j;
        while k < bytes.len() {
            match bytes[k] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        k += 1;
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }

        // ASCII '@', '{', '}' never appear inside a UTF-8 continuation byte,
        // so `start`/`k` are always valid char boundaries.
        let entry = content[start..k].trim();
        if !entry.is_empty() {
            entries.push(entry.to_string());
        }

        if depth != 0 {
            break; // unbalanced input — stop rather than loop forever
        }
        i = k;
    }

    entries
}

/// Extract the citation key of a single BibTeX entry: the text between the
/// first `{` and the first following `,` or `}`.
pub fn entry_key(entry: &str) -> Option<&str> {
    let open = entry.find('{')?;
    let rest = &entry[open + 1..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    let key = rest[..end].trim();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// Save BibTeX content to a file.
///
/// In append mode, entries whose key already exists in the file are skipped;
/// genuinely new entries are always appended (never dropped). In overwrite /
/// new-file mode the content is written verbatim.
pub fn save_bibtex(path: &Path, content: &str, append: bool) -> Result<()> {
    // Make sure the destination directory exists.
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    if append && path.exists() {
        let existing = fs::read_to_string(path)?;
        let existing_keys: HashSet<String> = split_bibtex_entries(&existing)
            .iter()
            .filter_map(|e| entry_key(e).map(str::to_string))
            .collect();

        // Keep every incoming entry that is not already present.
        let mut to_append: Vec<String> = Vec::new();
        for entry in split_bibtex_entries(content) {
            match entry_key(&entry) {
                Some(key) if existing_keys.contains(key) => {
                    eprintln!("Skipping duplicate entry: {}", key);
                }
                _ => to_append.push(entry),
            }
        }

        if to_append.is_empty() {
            eprintln!("No new entries to add (all duplicates)");
            return Ok(());
        }

        let mut file = OpenOptions::new().append(true).open(path)?;
        write!(file, "\n\n{}\n", to_append.join("\n\n"))?;
        eprintln!(
            "Appended {} new entries to {}",
            to_append.len(),
            path.display()
        );
    } else {
        fs::write(path, content)?;
        eprintln!(
            "Saved {} entries to {}",
            split_bibtex_entries(content).len(),
            path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_split_compact_entry_without_trailing_newline() {
        let content = "@article{Foo, year = {2020}}";
        let entries = split_bibtex_entries(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entry_key(&entries[0]), Some("Foo"));
    }

    #[test]
    fn test_split_multiple_entries() {
        let content = "@article{A,\n  title = {x}\n}\n@book{B,\n  title = {y}\n}";
        let entries = split_bibtex_entries(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entry_key(&entries[0]), Some("A"));
        assert_eq!(entry_key(&entries[1]), Some("B"));
    }

    #[test]
    fn test_split_entry_with_nested_braces() {
        let content = "@article{Nested,\n  title = {A {deeply} nested title}\n}";
        let entries = split_bibtex_entries(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entry_key(&entries[0]), Some("Nested"));
    }

    #[test]
    fn test_entry_key_handles_whitespace_and_no_fields() {
        assert_eq!(entry_key("@misc{ Key2024 ,\n}"), Some("Key2024"));
        assert_eq!(entry_key("@misc{OnlyKey}"), Some("OnlyKey"));
        assert_eq!(entry_key("not an entry"), None);
    }

    #[test]
    fn test_append_skips_duplicates_but_keeps_new_entries() {
        // Regression: compact entries (no `\n}`) used to be dropped entirely
        // whenever the payload also contained a duplicate.
        let dir = tempdir().unwrap();
        let path = dir.path().join("refs.bib");
        fs::write(&path, "@article{Old, year = {1999}}").unwrap();

        let content = "@article{Old, year = {1999}}\n@article{New, year = {2024}}";
        save_bibtex(&path, content, true).unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("@article{New"), "new entry must be appended");
        assert_eq!(
            saved.matches("@article{Old").count(),
            1,
            "duplicate entry must not be appended again"
        );
    }

    #[test]
    fn test_append_all_duplicates_writes_nothing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("refs.bib");
        fs::write(&path, "@article{Dup,\n  year = {2000}\n}").unwrap();
        let before = fs::read_to_string(&path).unwrap();

        save_bibtex(&path, "@article{Dup,\n  year = {2000}\n}", true).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn test_save_new_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bib");

        let content = "@article{Test2024,\n    author = {Test},\n}";
        save_bibtex(&path, content, false).unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("Test2024"));
    }

    #[test]
    fn test_save_creates_missing_parent_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/sub/refs.bib");
        save_bibtex(&path, "@article{X,\n}", false).unwrap();
        assert!(path.exists());
    }
}
