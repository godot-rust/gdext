/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Usage note:
// May require unlinking this crate from top-level workspace Cargo.toml, so that broken Cargo.toml can be
// overridden. It can help to call `cargo run -p repo-tweak` directly from this crate's directory.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// Manual input.
#[rustfmt::skip]
pub const GODOT_LATEST_PATCH_VERSIONS: &[&str] = &[
    // 4.0.4 no longer supported.
    // 4.1.4 no longer supported.
    "4.2.2",
    "4.3.0",
    "4.4.0",
    "4.5.0",
    "4.6.0",
    "4.7.0", // Upcoming.
];

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Use lib.rs as module; so we don't need another crate just for that.
// Lint #[allow(special_module_name)] is broken, thus rename.
#[path = "lib.rs"]
mod library;

fn main() {
    let workspace_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    sync_versions_recursive(workspace_dir, true);
}

fn sync_versions_recursive(parent_dir: &Path, top_level: bool) {
    // Iterate recursively
    for dir in parent_dir.read_dir().expect("read workspace dir") {
        let dir = dir.expect("read dir entry");
        let path = dir.path();

        if path.is_dir() {
            // Only recurse into `godot` and `godot-*` crates.
            if !top_level
                || path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("godot")
            {
                sync_versions_recursive(&path, false);
            }
        } else {
            // Is a file.
            if !matches!(path.extension(), Some(ext) if ext == "rs" || ext == "toml") {
                continue;
            }
            // println!("Check: {}", path.display());

            // Replace parts
            let content = std::fs::read_to_string(&path).expect("read file");

            let keys = ["include", "line", "pre", "post"];
            let ranges =
                library::find_repeated_ranges(&content, "[version-sync] [[", "]]", &keys, true);

            let mut last_pos = 0;
            if !ranges.is_empty() {
                println!("-> Replace: {}", path.display());
                println!("  -> Found {} ranges", ranges.len());

                let mut file = File::create(path).expect("create file");
                for m in ranges {
                    file.write_all(&content.as_bytes()[last_pos..m.start])
                        .expect("write file (before start)");

                    // Note: m.start..m.end is discarded and replaced with newly generated lines.
                    let (replaced, pre, post) = substitute_template(&m.key_values);
                    if let Some(pre) = pre {
                        write_newline(&mut file);
                        file.write_all(pre.as_bytes()).expect("write file (pre)");
                    }

                    for line in replaced {
                        // Write newline before.
                        write_newline(&mut file);
                        file.write_all(line.as_bytes())
                            .expect("write file (generated line)");
                    }

                    if let Some(post) = post {
                        write_newline(&mut file);
                        file.write_all(post.as_bytes()).expect("write file (post)");
                    }

                    file.write_all(&content.as_bytes()[m.end..m.after_end])
                        .expect("write file (after end)");

                    last_pos = m.after_end;
                }

                file.write_all(&content.as_bytes()[last_pos..])
                    .expect("write to file (end)");
            }
        }
    }
}

fn write_newline(file: &mut File) {
    file.write_all(b"\n").expect("write file (newline)")
}

/// For a given template, generate lines of content to be filled.
fn substitute_template(
    key_values: &HashMap<String, String>,
) -> (Vec<String>, Option<String>, Option<String>) {
    let template = key_values
        .get("line")
        .expect("version-sync: missing required [line] key");
    let template = apply_char_substitutions(template);

    let versions_max = latest_patch_versions();
    let mut applicable_versions = vec![];

    let default = "past+current".to_string();
    let parts = key_values.get("include").unwrap_or(&default).split('+');

    for part in parts {
        let current_minor = versions_max[versions_max.len() - 2].0;

        let filter: Box<dyn Fn(u8, u8) -> bool> = match part {
            "past" => Box::new(|m, _p| m < current_minor),
            "current" => Box::new(|m, _p| m == current_minor),
            "future" => Box::new(|m, _p| m > current_minor),
            "current.minor" => Box::new(|m, p| m == current_minor && p == 0),

            other => {
                panic!("version-sync: invalid value '{other}' for [include] key")
            }
        };

        for (minor, highest_patch) in versions_max.iter().copied() {
            for patch in 0..=highest_patch {
                if filter(minor, patch) {
                    applicable_versions.push((minor, patch));
                }
            }
        }
    }

    // Apply variable substitutions.
    let substituted = applicable_versions
        .into_iter()
        .map(|(minor, patch)| {
            if patch == 0 {
                template
                    .replace("$dotVersion", &format!("4.{minor}"))
                    .replace("$kebabVersion", &format!("4-{minor}"))
                    .replace("$snakeVersion", &format!("4_{minor}"))
                    .replace("$triple", &format!("(4, {minor}, 0)"))
            } else {
                template
                    .replace("$dotVersion", &format!("4.{minor}.{patch}"))
                    .replace("$kebabVersion", &format!("4-{minor}-{patch}"))
                    .replace("$snakeVersion", &format!("4_{minor}_{patch}"))
                    .replace("$triple", &format!("(4, {minor}, {patch})"))
            }
        })
        .collect();

    // Pre/post are needed because e.g. within #[cfg], no comments are possible.
    let pre = key_values.get("pre").map(|s| apply_char_substitutions(s));
    let post = key_values.get("post").map(|s| apply_char_substitutions(s));

    (substituted, pre, post)
}

fn apply_char_substitutions(s: &str) -> String {
    s.replace("\\t", "    ") // Not \t due to rustfmt.
        .replace("\\n", "\n")
}

fn latest_patch_versions() -> Vec<(u8, u8)> {
    GODOT_LATEST_PATCH_VERSIONS
        .iter()
        .map(|v| {
            let mut parts = v.split('.');
            let _major: u8 = parts.next().unwrap().parse().unwrap();
            let minor = parts.next().unwrap().parse().unwrap();
            let patch = parts.next().unwrap().parse().unwrap();
            (minor, patch)
        })
        .collect()
}
