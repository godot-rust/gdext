/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Type-safe access to [Godot's feature tags][tags].
//!
//! Feature tags describe the environment the engine was built for and runs in: the platform, the architecture, whether this is an
//! editor or an export template build, which texture compression formats are available, and so on. The engine exposes them through
//! [`OS::has_feature()`][has_feature], which takes the tag as a plain string -- easy to misspell, and nothing checks it at compile
//! time.
//!
//! [`FeatureTag`] offers the predefined tags as constants, so a typo becomes a compile error. Custom tags, e.g. ones declared in an
//! export preset or via `custom_features` in a `.gdextension` file, are still expressible with [`FeatureTag::user_defined()`].
//!
//! ```no_run
//! # use godot::prelude::*;
//! use godot::tools::{FeatureTag, has_feature};
//!
//! fn report_platform() {
//!     if has_feature(&FeatureTag::WINDOWS) {
//!         godot_print!("running on Windows");
//!     }
//! }
//! ```
//!
//! [tags]: https://docs.godotengine.org/en/stable/tutorials/export/feature_tags.html
//! [has_feature]: https://docs.godotengine.org/en/stable/classes/class_os.html#class-os-method-has-feature

use std::borrow::Cow;
use std::fmt;

use crate::builtin::GString;
use crate::classes::Os;
use crate::obj::Singleton;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public type

/// A tag describing a capability of the engine, as queried by [`OS::has_feature()`][crate::classes::Os::has_feature].
///
/// Use the associated constants for the predefined tags and [`user_defined()`][Self::user_defined] for your own. The tag names are
/// case-sensitive, exactly like the strings the engine expects.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FeatureTag(Cow<'static, str>);

impl FeatureTag {
    /// Creates a tag for a custom feature.
    ///
    /// This is meant for features that are not part of the predefined list, for example ones added by an export preset or the
    /// `custom_features` key of a `.gdextension` file. No validation is performed, the string is passed to the engine as-is.
    pub fn user_defined(tag: &str) -> Self {
        Self(Cow::Owned(tag.to_owned()))
    }

    /// Returns the string that is passed to the engine.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FeatureTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for FeatureTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FeatureTag({:?})", self.0.as_ref())
    }
}

impl From<&str> for FeatureTag {
    fn from(tag: &str) -> Self {
        Self::user_defined(tag)
    }
}

impl From<&FeatureTag> for GString {
    fn from(tag: &FeatureTag) -> Self {
        GString::from(tag.as_str())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Predefined tags

/// The predefined feature tags, as documented in the [Godot manual][tags].
///
/// [tags]: https://docs.godotengine.org/en/stable/tutorials/export/feature_tags.html
impl FeatureTag {
    // Platform.
    pub const ANDROID: Self = Self::predefined("android");
    pub const BSD: Self = Self::predefined("bsd");
    pub const IOS: Self = Self::predefined("ios");
    pub const LINUX: Self = Self::predefined("linux");
    pub const LINUXBSD: Self = Self::predefined("linuxbsd");
    pub const MACOS: Self = Self::predefined("macos");
    pub const VISIONOS: Self = Self::predefined("visionos");
    pub const WINDOWS: Self = Self::predefined("windows");
    pub const WEB: Self = Self::predefined("web");

    // Platform the project is running in a browser on.
    pub const WEB_ANDROID: Self = Self::predefined("web_android");
    pub const WEB_IOS: Self = Self::predefined("web_ios");
    pub const WEB_LINUXBSD: Self = Self::predefined("web_linuxbsd");
    pub const WEB_MACOS: Self = Self::predefined("web_macos");
    pub const WEB_WINDOWS: Self = Self::predefined("web_windows");

    // Platform type.
    pub const MOBILE: Self = Self::predefined("mobile");
    pub const PC: Self = Self::predefined("pc");

    // Architecture and bitness.
    pub const BITS_32: Self = Self::predefined("32");
    pub const BITS_64: Self = Self::predefined("64");
    pub const X86: Self = Self::predefined("x86");
    pub const X86_32: Self = Self::predefined("x86_32");
    pub const X86_64: Self = Self::predefined("x86_64");
    pub const ARM: Self = Self::predefined("arm");
    pub const ARM32: Self = Self::predefined("arm32");
    pub const ARM64: Self = Self::predefined("arm64");
    pub const RISCV: Self = Self::predefined("riscv");
    pub const RV64: Self = Self::predefined("rv64");
    pub const PPC: Self = Self::predefined("ppc");
    pub const PPC32: Self = Self::predefined("ppc32");
    pub const PPC64: Self = Self::predefined("ppc64");
    pub const WASM: Self = Self::predefined("wasm");
    pub const WASM32: Self = Self::predefined("wasm32");
    pub const WASM64: Self = Self::predefined("wasm64");

    // Build type.
    pub const DEBUG: Self = Self::predefined("debug");
    pub const RELEASE: Self = Self::predefined("release");
    pub const EDITOR: Self = Self::predefined("editor");
    pub const EDITOR_HINT: Self = Self::predefined("editor_hint");
    pub const EDITOR_RUNTIME: Self = Self::predefined("editor_runtime");
    pub const TEMPLATE: Self = Self::predefined("template");

    // Numeric precision.
    pub const SINGLE: Self = Self::predefined("single");
    pub const DOUBLE: Self = Self::predefined("double");

    // Threading.
    pub const THREADS: Self = Self::predefined("threads");
    pub const NOTHREADS: Self = Self::predefined("nothreads");

    // Texture compression.
    pub const ETC: Self = Self::predefined("etc");
    pub const ETC2: Self = Self::predefined("etc2");
    pub const S3TC: Self = Self::predefined("s3tc");

    // Export modes.
    pub const DEDICATED_SERVER: Self = Self::predefined("dedicated_server");
    pub const MOVIE: Self = Self::predefined("movie");
    pub const SHADER_BAKER: Self = Self::predefined("shader_baker");

    const fn predefined(tag: &'static str) -> Self {
        Self(Cow::Borrowed(tag))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Querying

/// Returns whether the engine was built with the given feature, i.e. [`OS::has_feature()`][crate::classes::Os::has_feature].
///
/// Prefer this over passing a string literal, since the tag is checked at compile time.
///
/// # Panics
/// If the engine is not initialized yet, or has already been deinitialized.
pub fn has_feature(tag: &FeatureTag) -> bool {
    let os = Os::singleton();
    os.has_feature(tag.as_str())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    /// Every predefined tag, so the checks below cover the whole list.
    const PREDEFINED: &[FeatureTag] = &[
        FeatureTag::ANDROID,
        FeatureTag::BSD,
        FeatureTag::IOS,
        FeatureTag::LINUX,
        FeatureTag::LINUXBSD,
        FeatureTag::MACOS,
        FeatureTag::VISIONOS,
        FeatureTag::WINDOWS,
        FeatureTag::WEB,
        FeatureTag::WEB_ANDROID,
        FeatureTag::WEB_IOS,
        FeatureTag::WEB_LINUXBSD,
        FeatureTag::WEB_MACOS,
        FeatureTag::WEB_WINDOWS,
        FeatureTag::MOBILE,
        FeatureTag::PC,
        FeatureTag::BITS_32,
        FeatureTag::BITS_64,
        FeatureTag::X86,
        FeatureTag::X86_32,
        FeatureTag::X86_64,
        FeatureTag::ARM,
        FeatureTag::ARM32,
        FeatureTag::ARM64,
        FeatureTag::RISCV,
        FeatureTag::RV64,
        FeatureTag::PPC,
        FeatureTag::PPC32,
        FeatureTag::PPC64,
        FeatureTag::WASM,
        FeatureTag::WASM32,
        FeatureTag::WASM64,
        FeatureTag::DEBUG,
        FeatureTag::RELEASE,
        FeatureTag::EDITOR,
        FeatureTag::EDITOR_HINT,
        FeatureTag::EDITOR_RUNTIME,
        FeatureTag::TEMPLATE,
        FeatureTag::SINGLE,
        FeatureTag::DOUBLE,
        FeatureTag::THREADS,
        FeatureTag::NOTHREADS,
        FeatureTag::ETC,
        FeatureTag::ETC2,
        FeatureTag::S3TC,
        FeatureTag::DEDICATED_SERVER,
        FeatureTag::MOVIE,
        FeatureTag::SHADER_BAKER,
    ];

    #[test]
    fn predefined_tags_are_lowercase_ascii() {
        for tag in PREDEFINED {
            let s = tag.as_str();
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "feature tag {s:?} is not in the format the engine expects"
            );
        }
    }

    #[test]
    fn predefined_tags_are_unique() {
        for (i, a) in PREDEFINED.iter().enumerate() {
            for b in PREDEFINED.iter().skip(i + 1) {
                assert_ne!(a, b, "duplicate feature tag {a}");
            }
        }
    }

    #[test]
    fn predefined_tags_match_their_names() {
        // Spot checks that constants weren't wired to the wrong string.
        assert_eq!(FeatureTag::BITS_64.as_str(), "64");
        assert_eq!(FeatureTag::X86_64.as_str(), "x86_64");
        assert_eq!(FeatureTag::EDITOR_HINT.as_str(), "editor_hint");
        assert_eq!(FeatureTag::WINDOWS.as_str(), "windows");
    }

    #[test]
    fn user_defined_keeps_custom_tags() {
        let tag = FeatureTag::user_defined("my_custom_feature");
        assert_eq!(tag.as_str(), "my_custom_feature");
        assert_eq!(tag.to_string(), "my_custom_feature");
        assert_eq!(tag, FeatureTag::from("my_custom_feature"));
    }

    #[test]
    fn display_and_debug_are_readable() {
        assert_eq!(FeatureTag::LINUX.to_string(), "linux");
        assert_eq!(format!("{:?}", FeatureTag::LINUX), r#"FeatureTag("linux")"#);
    }
}
