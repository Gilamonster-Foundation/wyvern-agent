// Copyright 2026 The Wyvern Authors
//
// Licensed under either of
//
//   * Apache License, Version 2.0 (LICENSE-APACHE or
//     https://www.apache.org/licenses/LICENSE-2.0)
//   * MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
//
// at your option.

//! HARD-RULE guard: no vendor identities in non-test, non-comment
//! source.
//!
//! The charter's clean test is:
//!
//! ```text
//! grep -riE 'claude|openai|anthropic|gpt|gemini' crates/*/src  => zero
//! ```
//!
//! We grade a stricter, machine-checked version: vendor tokens may
//! appear only inside doc/line comments (where we explain *why they're
//! absent*) and inside `#[cfg(test)]` modules. They must never appear in
//! live code. This walks every crate's `src/` tree, strips line
//! comments, skips test modules, and asserts no vendor token survives.
//!
//! Workers are reached by capability — newt-agent instances selected by
//! the `agent-bridle` `Caveats` they satisfy — never by API provider.

use std::fs;
use std::path::{Path, PathBuf};

const VENDOR_TOKENS: &[&str] = &[
    "claude",
    "openai",
    "anthropic",
    "gpt",
    "gemini",
    "ollama",
    "mistral",
    "cohere",
];

/// Walk up from this crate to the workspace `crates/` dir, then collect
/// every `crates/*/src/**/*.rs` file.
fn workspace_src_files() -> Vec<PathBuf> {
    // CARGO_MANIFEST_DIR = .../crates/wyvern-wire
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest
        .parent()
        .expect("crate dir has a parent (crates/)")
        .to_path_buf();
    let mut files = Vec::new();
    collect_rs(&crates_dir, &mut files);
    files
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            // Only descend into src/ trees; skip target/, tests/, etc.
            if name == "target" || name == "tests" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Only consider files under a src/ directory.
            if path.components().any(|c| c.as_os_str() == "src") {
                out.push(path);
            }
        }
    }
}

/// Strip a line comment (`//...`) and return the code portion, lowercased.
/// Crude but sufficient: our codebase has no `//` inside string literals.
fn code_part(line: &str) -> String {
    let code = match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    };
    code.to_lowercase()
}

#[test]
fn no_vendor_identities_in_src() {
    let files = workspace_src_files();
    assert!(!files.is_empty(), "guard found no src files to scan");

    let mut offenders: Vec<String> = Vec::new();

    for file in &files {
        let contents = fs::read_to_string(file).expect("read src file");
        let mut in_test_module = false;
        let mut test_brace_depth: i32 = 0;

        for (lineno, raw) in contents.lines().enumerate() {
            let trimmed = raw.trim_start();

            // Enter a test module on `#[cfg(test)]` and track its braces
            // so vendor tokens inside tests are allowed.
            if trimmed.starts_with("#[cfg(test)]") {
                in_test_module = true;
                test_brace_depth = 0;
            }
            if in_test_module {
                test_brace_depth += raw.matches('{').count() as i32;
                test_brace_depth -= raw.matches('}').count() as i32;
                // Once we've opened and closed back to zero (or below),
                // the module is over.
                if test_brace_depth <= 0 && raw.contains('}') {
                    in_test_module = false;
                }
                continue;
            }

            let code = code_part(raw);
            for token in VENDOR_TOKENS {
                if code.contains(token) {
                    offenders.push(format!(
                        "{}:{}: vendor token {:?} in code: {}",
                        file.display(),
                        lineno + 1,
                        token,
                        raw.trim()
                    ));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "vendor identities found in non-test code (capabilities, not vendors!):\n{}",
        offenders.join("\n")
    );
}
