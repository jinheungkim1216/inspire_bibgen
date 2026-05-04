use crate::Result;
use regex::Regex;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Extract BibTeX entry keys from content
fn extract_bibtex_keys(content: &str) -> HashSet<String> {
    let key_regex = Regex::new(r"@\w+\{([^,]+),").unwrap();
    key_regex
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .collect()
}

/// Save BibTeX content to a file
pub fn save_bibtex(path: &Path, content: &str, append: bool) -> Result<()> {
    if append && path.exists() {
        // Read existing content to check for duplicates
        let existing = fs::read_to_string(path)?;
        let existing_keys = extract_bibtex_keys(&existing);
        let new_keys = extract_bibtex_keys(content);

        // Filter out duplicate entries
        let mut filtered_content = String::new();
        let entry_regex = Regex::new(r"(@\w+\{[^,]+,[\s\S]*?\n\})").unwrap();

        for cap in entry_regex.captures_iter(content) {
            if let Some(entry) = cap.get(1) {
                let entry_str = entry.as_str();
                // Extract the key from this entry
                if let Some(key_cap) = Regex::new(r"@\w+\{([^,]+),")
                    .unwrap()
                    .captures(entry_str)
                {
                    if let Some(key) = key_cap.get(1) {
                        let key_str = key.as_str().trim();
                        if !existing_keys.contains(key_str) {
                            if !filtered_content.is_empty() {
                                filtered_content.push_str("\n\n");
                            }
                            filtered_content.push_str(entry_str);
                        } else {
                            eprintln!("Skipping duplicate entry: {}", key_str);
                        }
                    }
                }
            }
        }

        // Check if content has entries that don't match the regex
        // (fallback for simple single-entry case)
        if filtered_content.is_empty() && !content.trim().is_empty() {
            let content_keys = extract_bibtex_keys(content);
            let has_duplicates = content_keys.iter().any(|k| existing_keys.contains(k));
            if !has_duplicates {
                filtered_content = content.trim().to_string();
            }
        }

        if filtered_content.is_empty() {
            println!("No new entries to add (all duplicates)");
            return Ok(());
        }

        // Append to existing file
        let mut file = OpenOptions::new().append(true).open(path)?;
        writeln!(file)?; // Add newline separator
        writeln!(file)?;
        write!(file, "{}", filtered_content)?;

        let new_count = new_keys.difference(&existing_keys).count();
        println!(
            "Appended {} new entries to {}",
            new_count,
            path.display()
        );
    } else {
        // Overwrite or create new file
        fs::write(path, content)?;
        let count = extract_bibtex_keys(content).len();
        println!(
            "Saved {} entries to {}",
            count,
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
    fn test_extract_bibtex_keys() {
        let content = r#"
@article{Einstein1905,
    author = {Einstein, Albert},
    title = {On the Electrodynamics of Moving Bodies},
}

@book{Feynman1965,
    author = {Feynman, Richard},
    title = {The Feynman Lectures},
}
"#;
        let keys = extract_bibtex_keys(content);
        assert!(keys.contains("Einstein1905"));
        assert!(keys.contains("Feynman1965"));
        assert_eq!(keys.len(), 2);
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
}
