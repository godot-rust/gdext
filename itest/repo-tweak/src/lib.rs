/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

#[derive(Debug)]
pub struct Match {
    /// Position before pattern start marker.
    #[allow(dead_code)] // False-positive, regression introduced in Rust 1.79.
    pub before_start: usize,

    /// Position at the beginning of the repetition (after marker + keys).
    pub start: usize,

    /// Position 1 past the end of the repetition.
    pub end: usize,

    /// Position after the end pattern marker.
    pub after_end: usize,

    /// Extra keys following the start pattern marker.
    pub key_values: HashMap<String, String>,
}

pub fn find_repeated_ranges(
    entire: &str,
    start_pat: &str,
    end_pat: &str,
    keys: &[&str],
    retain_end_pat: bool,
) -> Vec<Match> {
    let mut search_start = 0;
    let mut found = vec![];

    while let Some(start) = entire[search_start..].find(start_pat) {
        let before_start = search_start + start;
        let start = before_start + start_pat.len();

        let mut key_values = HashMap::new();

        let Some(end) = entire[start..].find(end_pat) else {
            panic!("unmatched start pattern '{start_pat}' without end");
        };

        let end = start + end;
        let end = if retain_end_pat {
            // Rewind until previous newline.
            entire[..end + 1].rfind('\n').unwrap_or(end)
        } else {
            end
        };

        let after_end = end + end_pat.len();

        let within = &entire[start..end];
        // println!("Within: <<{within}>>");

        let mut after_keys = start;
        for key in keys {
            let key_fmt = format!("[{key}] ");
            // println!("  Find '{key_fmt}' -> {:?}", within.find(&key_fmt));

            let Some(pos) = within.find(&key_fmt) else {
                continue;
            };

            let pos = pos + key_fmt.len();

            // Read until end of line -> that's the value.
            let eol = within[pos..]
                .find(['\n', '\r'])
                .unwrap_or_else(|| panic!("unterminated line for key '{key}'"));

            let value = &within[pos..pos + eol];
            key_values.insert(key.to_string(), value.to_string());

            after_keys = after_keys.max(start + pos + eol);
        }

        found.push(Match {
            before_start,
            start: after_keys,
            end,
            after_end,
            key_values,
        });
        search_start = after_end;
    }

    found
}
