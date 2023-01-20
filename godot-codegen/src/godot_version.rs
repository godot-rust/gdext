/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//#![allow(unused_variables, dead_code)]

use regex::Regex;
use std::error::Error;

pub struct GodotVersion {
    /// the original string (trimmed, stripped of text around)
    pub full_string: String,

    pub major: u8,
    pub minor: u8,

    /// 0 if none
    pub patch: u8,

    /// alpha|beta|dev|stable
    pub stability: String,

    /// Git revision 'custom_build.{rev}' or '{official}.rev', if available
    pub custom_rev: Option<String>,
}

pub fn parse_godot_version(version_str: &str) -> Result<GodotVersion, Box<dyn Error>> {
    let regex = Regex::new(
        //  major  minor     [patch]                                              official|custom_build|gentoo
        //  v      v         v                                                    v
        r#"(\d+)\.(\d+)(?:\.(\d+))?\.(alpha|beta|dev|stable)[0-9]*\.(?:mono\.)?(?:[a-z_]+\.([a-f0-9]+)|official)"#,
    )?;

    let fail = || format!("Version substring cannot be parsed: `{version_str}`");
    let caps = regex.captures(version_str).ok_or_else(fail)?;

    Ok(GodotVersion {
        full_string: caps.get(0).unwrap().as_str().to_string(),
        major: caps.get(1).ok_or_else(fail)?.as_str().parse::<u8>()?,
        minor: caps.get(2).ok_or_else(fail)?.as_str().parse::<u8>()?,
        patch: caps
            .get(3)
            .map(|m| m.as_str().parse::<u8>())
            .transpose()?
            .unwrap_or(0),
        stability: caps.get(4).ok_or_else(fail)?.as_str().to_string(),
        custom_rev: caps.get(5).map(|m| m.as_str().to_string()),
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
        ("3.4.stable.official.206ba70f4", 3, 4, 0, "stable",  s("206ba70f4")),
        ("3.4.1.stable.official.aa1b95889", 3, 4, 1, "stable",  s("aa1b95889")),
        ("3.5.beta.custom_build.837f2c5f8", 3, 5, 0, "beta", s("837f2c5f8")),
        ("4.0.beta8.gentoo.45cac42c0", 4, 0, 0, "beta", s("45cac42c0")),
        ("4.0.dev.custom_build.e7e9e663b", 4, 0, 0, "dev", s("e7e9e663b")),
        ("4.0.alpha.custom_build.faddbcfc0", 4, 0, 0, "alpha", s("faddbcfc0")),
        ("4.0.beta8.mono.custom_build.b28ddd918", 4, 0, 0, "beta", s("b28ddd918")),
    ];

    let bad_versions = [
        "4.0.unstable.custom_build.e7e9e663b", // 'unstable'
        "4.0.3.custom_build.e7e9e663b",        // no stability
        "3.stable.official.206ba70f4",         // no minor
        "4.0.alpha.custom_build",              // no rev after 'custom_build' (this is allowed for 'official' however)
    ];

    for (full, major, minor, patch, stability, custom_rev) in good_versions {
        let parsed: GodotVersion = parse_godot_version(full).unwrap();
        assert_eq!(parsed.major, major);
        assert_eq!(parsed.minor, minor);
        assert_eq!(parsed.patch, patch);
        assert_eq!(parsed.stability, stability);
        assert_eq!(parsed.custom_rev, custom_rev);
        assert_eq!(parsed.full_string, full);
    }

    for full in bad_versions {
        let parsed = parse_godot_version(full);
        assert!(parsed.is_err());
    }
}
