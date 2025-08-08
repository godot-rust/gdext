/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file is explicitly included in unit tests
// while all the functions included are used only with `custom-api` and `custom-api-json` features.
#![cfg_attr(not(feature = "api-custom"), allow(unused_variables, dead_code))]

use std::error::Error;
use std::str::FromStr;

use regex::{Captures, Regex};

use crate::GodotVersion;

pub fn parse_godot_version(version_str: &str) -> Result<GodotVersion, Box<dyn Error>> {
    // Format of the string emitted by `godot --version`:
    // https://github.com/godot-rust/gdext/issues/118#issuecomment-1465748123
    // We assume that it's on a line of its own, but it may be surrounded by other lines.
    let regex = Regex::new(
        r"(?xm)
        # x: ignore whitespace and allow line comments (starting with `#`)
        # m: multi-line mode, ^ and $ match start and end of line
        ^
        (?P<major>\d+)
        \.(?P<minor>\d+)
        # Patch version is omitted if it's zero.
        (?:\.(?P<patch>\d+))?
        # stable|dev|alpha|beta|rc12|... Can be set through an env var when the engine is built.
        \.(?P<status>[^.]+)
        # Capture both module config and build, could be multiple components:
        # mono|official|custom_build|gentoo|arch_linux|...
        # Notice +? for non-greedy match.
        (\.[^.]+)+?
        # Git commit SHA1, currently truncated to 9 chars, but accept the full thing
        (?:\.(?P<custom_rev>[a-f0-9]{9,40}))?
        # Optional newline printed in some systems (e.g. Arch Linux, see #416)
        (?:\\n)?
        $
        ",
    )?;

    let fail = || format!("Version substring cannot be parsed: `{version_str}`");
    let caps = regex.captures(version_str).ok_or_else(fail)?;

    Ok(GodotVersion {
        full_string: caps.get(0).unwrap().as_str().trim().to_string(),
        major: cap(&caps, "major")?.unwrap(),
        minor: cap(&caps, "minor")?.unwrap(),
        patch: cap(&caps, "patch")?.unwrap_or(0),
        status: cap(&caps, "status")?.unwrap(),
        custom_rev: cap(&caps, "custom_rev")?,
    })
}

pub(crate) fn validate_godot_version(godot_version: &GodotVersion) {
    assert_eq!(
        godot_version.major, 4,
        "Only Godot versions with major version 4 are supported; found version {}.",
        godot_version.full_string
    );

    assert!(
        godot_version.minor > 0,
        "Godot 4.0 is no longer supported by godot-rust; found version {}.",
        godot_version.full_string
    );
}

/// Extracts and parses a named capture group from a regex match.
fn cap<T: FromStr>(caps: &Captures, key: &str) -> Result<Option<T>, Box<dyn Error>> {
    caps.name(key)
        .map(|m| m.as_str().parse())
        .transpose()
        .map_err(|_| {
            format!(
                "Version string cannot be parsed: `{}`",
                caps.get(0).unwrap().as_str()
            )
            .into()
        })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
#[rustfmt::skip]
fn test_godot_versions() {
    fn s(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    let good_versions = [
        ("3.0.stable.official", 3, 0, 0, "stable", None),
        ("3.0.1.stable.official", 3, 0, 1, "stable", None),
        ("3.2.stable.official", 3, 2, 0, "stable", None),
        ("3.37.stable.official", 3, 37, 0, "stable", None),
        ("3.4.stable.official.206ba70f4", 3, 4, 0, "stable", s("206ba70f4")),
        ("3.4.1.stable.official.aa1b95889", 3, 4, 1, "stable", s("aa1b95889")),
        ("3.5.beta.custom_build.837f2c5f8", 3, 5, 0, "beta", s("837f2c5f8")),
        ("4.0.beta8.gentoo.45cac42c0", 4, 0, 0, "beta8", s("45cac42c0")),
        ("4.0.dev.custom_build.e7e9e663b", 4, 0, 0, "dev", s("e7e9e663b")),
        ("4.0.alpha.custom_build.faddbcfc0", 4, 0, 0, "alpha", s("faddbcfc0")),
        ("4.0.beta8.mono.custom_build.b28ddd918", 4, 0, 0, "beta8", s("b28ddd918")),
        ("4.0.rc1.official.8843d9ad3", 4, 0, 0, "rc1", s("8843d9ad3")),
        ("4.0.stable.arch_linux", 4, 0, 0, "stable", None),
        ("4.1.1.stable.arch_linux\n", 4, 1, 1, "stable", None),
        // Output from 4.0.stable on macOS in debug mode:
        // https://github.com/godotengine/godot/issues/74906
        ("arguments
0: /Users/runner/work/_temp/godot_bin/godot.macos.editor.dev.x86_64
1: --version
Current path: /Users/runner/work/gdext/gdext/godot-core
4.1.dev.custom_build.79454bfd3", 4, 1, 0, "dev", s("79454bfd3")),
    ];

    let bad_versions = [
        "Godot Engine v4.0.stable.arch_linux - https://godotengine.org", // Surrounding cruft
        "3.stable.official.206ba70f4", // No minor version
        "4.0.stable", // No build type
    ];

    for (full, major, minor, patch, status, custom_rev) in good_versions {
        let expected = GodotVersion {
            // Version line is last in every test at the moment.
            full_string: full.lines().last().unwrap().trim().to_owned(),
            major,
            minor,
            patch,
            status: status.to_owned(),
            custom_rev,
        };
        let parsed: GodotVersion = parse_godot_version(full).unwrap();
        assert_eq!(parsed, expected, "{full}");
    }

    for full in bad_versions {
        let parsed = parse_godot_version(full);
        assert!(parsed.is_err(), "{}", full);
    }
}
