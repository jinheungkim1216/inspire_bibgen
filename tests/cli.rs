//! End-to-end CLI tests that exercise behaviour without network access.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_inspire_bibgen"))
        .args(args)
        .output()
        .expect("failed to run inspire_bibgen")
}

#[test]
fn invalid_identifiers_are_skipped_and_explicitly_listed() {
    // Both identifiers are unparseable. The first must NOT abort the batch:
    // every failed key must be named explicitly in the closing summary, and
    // the command exits non-zero because nothing was retrieved.
    let output = run(&["@@@bad,###worse"]);
    assert!(
        !output.status.success(),
        "command should fail when nothing is retrieved"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("could not be retrieved"),
        "expected an explicit missing-keys summary; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("@@@bad") && stderr.contains("###worse"),
        "both failed keys must be named; stderr was:\n{stderr}"
    );
}

#[test]
fn missing_tex_file_reports_file_not_found() {
    // A `.tex` argument that does not exist should be a clear file error,
    // not a misleading "invalid identifier" error.
    let output = run(&["definitely_missing_file_xyz.tex"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "expected a file-not-found error; stderr was:\n{stderr}"
    );
}

#[test]
fn key_flag_conflicts_with_output_file() {
    // `--key` prints texkeys to stdout; writing keys into a .bib makes no sense.
    let output = run(&["1706.03762", "--key", "-o", "refs.bib"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--output") && stderr.contains("--key"),
        "expected a clap conflict error naming both flags; stderr was:\n{stderr}"
    );
}

#[test]
fn key_flag_with_only_invalid_identifiers_fails_with_empty_stdout() {
    let output = run(&["--key", "@@@bad"]);
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "nothing should be printed to stdout on failure"
    );
}
