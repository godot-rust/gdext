/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Macros for translation.

pub use crate::{tr, tr_n};

/// A convenience macro for using the [`Object::tr()`](crate::classes::Object::tr()) and [`Object::tr_ex()`](crate::classes::Object::tr_ex())
///  methods.
///
/// Takes a format string literal, with optional arguments. Optionally, `context` for potentially ambiguous words can be
/// added before the format arguments, separated with a `;`.
///
/// Using named or positional parameters instead of `{}` may make it easier to use dynamic formatting once gdext supports it:
/// ```no_run
/// # #[macro_use] extern crate godot;
/// # use godot::builtin::Vector2i;
/// # let a = Vector2i { x: 0, y: 0 };
/// # let b = Vector2i { x: 0, y: 0 };
/// # let context = "context";
/// use godot::tools::tr;
///
/// // Good.
/// tr!(context; "{a} is a {b}"); // inlined, with context
/// tr!("{0} is a {1}", a, b); // positional, without context
/// tr!("{c} is a {d}", c = a.x, d = b.y); // named (inlining not possible here)
///
/// // Not as good, much more fragile.
/// tr!("{} is a {}", a, b);
/// ```
/// The methods are called from the [`Engine`](crate::classes::Engine) singleton.
///
/// A literal context must be a string literal; non-string literals such as `tr!(true; "msg")` are rejected, since they cannot be stored as a
/// `msgctxt`. With the `register-translations` feature, each invocation additionally self-registers its string for static extraction via
/// [`collect_translations()`](crate::tools::collect_translations)/[`write_pot()`](crate::tools::write_pot).
///
/// Note that the string is formatted *before* the lookup, so an invocation with arguments registers the format string but looks up the
/// formatted result. Translate a message without arguments and format afterwards, if you need both.
///
/// See also: [Translation contexts](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html#translation-contexts)
/// in Godot.
#[macro_export]
macro_rules! tr {
    ($fmt:literal $(, $($args:tt)*)?) => {{
        $crate::__tr_self_register!(
            $fmt,
            ::core::option::Option::None,
            ::core::option::Option::None,
            file!(),
            line!()
        );

        let msg = format!($fmt $(, $($args)*)?);

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton().tr(&msg)
    }};

    // Literal context: registered so it ends up as `msgctxt` during extraction.
    ($context:literal; $fmt:literal $(, $($args:tt)*)?) => {{
        // A literal context must be a string literal, so it can be stored as a `msgctxt`. This guard rejects non-string literals
        // (e.g. `tr!(true; "msg")`) uniformly, regardless of the `register-translations` feature.
        const _LITERAL_CONTEXT_MUST_BE_STR: &str = $context;

        $crate::__tr_self_register!(
            $fmt,
            ::core::option::Option::None,
            ::core::option::Option::Some($context),
            file!(),
            line!()
        );

        let msg = format!($fmt $(, $($args)*)?);

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_ex(&msg)
            .context($context)
            .done()
    }};

    // Non-literal context: can't be extracted statically, so it is registered without context.
    ($context:expr_2021; $fmt:literal $(, $($args:tt)*)?) => {{
        $crate::__tr_self_register!(
            $fmt,
            ::core::option::Option::None,
            ::core::option::Option::None,
            file!(),
            line!()
        );

        let msg = format!($fmt $(, $($args)*)?);
        let context = format!("{}", $context);

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_ex(&msg)
            .context(&context)
            .done()
    }};
}

/// A convenience macro for using the [`Object::tr_n()`](crate::classes::Object::tr_n()) and
/// [`Object::tr_n_ex()`](crate::classes::Object::tr_n_ex()) methods.
///
/// Allows for the use of format strings with arbitrary arguments. `n` is given prior to the format string, followed by `;`.
/// Optionally, `context` for potentially ambiguous words can be added with `,` after `n` and before `;`.
///
/// Using named or positional parameters instead of `{}` may make it easier to use dynamic formatting once gdext supports it:
/// ```no_run
/// # #[macro_use] extern crate godot;
/// # use godot::builtin::Vector2i;
/// # let a = Vector2i { x: 0, y: 0 };
/// # let b = Vector2i { x: 0, y: 0 };
/// # let context = "context";
/// # let n = 2;
/// use godot::tools::tr_n;
///
/// // Good.
/// tr_n!(n, context; "{a} is a {b}", "{a}s are {b}s"); // inlined, with context
/// tr_n!(n; "{0} is a {1}", "{0}s are {1}s", a, b); // positional, without context
/// tr_n!(n; "{c} is a {d}", "{c}s are {d}s", c = a.x, d = b.y); // named (inlining not possible here)
///
/// // Not as good, much more fragile.
/// // Additionally, such syntax requires that BOTH format strings use ALL listed arguments.
/// tr_n!(n; "{} is a {}", "{}s are {}s", a, b);
/// ```
/// The methods are called from the [`Engine`](crate::classes::Engine) singleton.
///
/// A literal context must be a string literal; non-string literals such as `tr_n!(n, true; "one", "many")` are rejected, since they cannot be
/// stored as a `msgctxt`. With the `register-translations` feature, each invocation additionally self-registers its strings for static
/// extraction; see [`tr!`] for that and for the interaction with format arguments.
///
/// See also: [Translation contexts](https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html#translation-contexts)
/// in Godot.
#[macro_export]
macro_rules! tr_n {
    ($n:expr_2021; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {{
        $crate::__tr_self_register!(
            $singular,
            ::core::option::Option::Some($plural),
            ::core::option::Option::None,
            file!(),
            line!()
        );

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_n(
                &format!($singular$(, $($args)*)?),
                &format!($plural$(, $($args)*)?),
                $n,
            )
    }};

    // Literal context: registered so it ends up as `msgctxt` during extraction.
    ($n:expr_2021, $context:literal; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {{
        // A literal context must be a string literal, so it can be stored as a `msgctxt`. This guard rejects non-string literals
        // (e.g. `tr_n!(n, true; "one", "many")`) uniformly, regardless of the `register-translations` feature.
        const _LITERAL_CONTEXT_MUST_BE_STR: &str = $context;

        $crate::__tr_self_register!(
            $singular,
            ::core::option::Option::Some($plural),
            ::core::option::Option::Some($context),
            file!(),
            line!()
        );

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_n_ex(
                &format!($singular$(, $($args)*)?),
                &format!($plural$(, $($args)*)?),
                $n,
            )
            .context($context)
            .done()
    }};

    // Non-literal context: can't be extracted statically, so it is registered without context.
    ($n:expr_2021, $context:expr_2021; $singular:literal, $plural:literal $(, $($args:tt)*)?) => {{
        $crate::__tr_self_register!(
            $singular,
            ::core::option::Option::Some($plural),
            ::core::option::Option::None,
            file!(),
            line!()
        );

        <$crate::classes::Engine as $crate::obj::Singleton>::singleton()
            .tr_n_ex(
                &format!($singular$(, $($args)*)?),
                &format!($plural$(, $($args)*)?),
                $n,
            )
            .context(&format!("{}", $context))
            .done()
    }};
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Self-registration of translation strings ("shards").

// `__tr_self_register!` is defined twice with mutually exclusive `cfg`s. This way, the `register-translations` feature is evaluated in
// *this* crate (godot-core), not in the user crate where `tr!` is expanded -- a `#[cfg]` written inside `tr!`'s own body would refer to
// the wrong crate's features.

/// Registers a translation string as a shard. Internal helper for [`tr!`] and [`tr_n!`]; not part of the public API.
#[doc(hidden)]
#[cfg(feature = "register-translations")]
#[macro_export]
macro_rules! __tr_self_register {
    ($msgid:expr, $plural:expr, $context:expr, $file:expr, $line:expr) => {
        $crate::sys::shard_add!(
            $crate::private::__GODOT_TRANSLATIONS_REGISTRY;
            $crate::private::TranslationShard::new($msgid, $plural, $context, $file, $line)
        );
    };
}

/// No-op variant used when the `register-translations` feature is disabled.
#[doc(hidden)]
#[cfg(not(feature = "register-translations"))]
#[macro_export]
macro_rules! __tr_self_register {
    ($($args:tt)*) => {};
}

#[cfg(feature = "register-translations")]
pub use registry::{TranslationShard, collect_translations, write_pot};

#[cfg(feature = "register-translations")]
mod registry {
    use std::collections::BTreeMap;

    /// A single translation string collected from a [`tr!`](crate::tools::tr) or [`tr_n!`](crate::tools::tr_n) invocation
    /// through self-registration.
    ///
    /// Only available with the `register-translations` feature. Instances are created automatically by the macros and gathered via
    /// [`collect_translations()`]; you should not construct them manually.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TranslationShard {
        /// The (singular) source string, i.e. the gettext `msgid`. Always a string literal.
        pub msgid: &'static str,

        /// The plural source string (gettext `msgid_plural`), present for `tr_n!` invocations.
        pub plural: Option<&'static str>,

        /// The translation context (gettext `msgctxt`), if a string literal was provided.
        ///
        /// Non-literal contexts cannot be extracted statically and are stored as `None`.
        pub context: Option<&'static str>,

        /// Source file in which the macro was invoked.
        pub file: &'static str,

        /// Source line on which the macro was invoked.
        pub line: u32,
    }

    impl TranslationShard {
        /// Creates a new shard. Called by the `tr!`/`tr_n!` macros; not meant for manual use.
        #[doc(hidden)]
        pub const fn new(
            msgid: &'static str,
            plural: Option<&'static str>,
            context: Option<&'static str>,
            file: &'static str,
            line: u32,
        ) -> Self {
            Self {
                msgid,
                plural,
                context,
                file,
                line,
            }
        }
    }

    /// Collects all translation strings registered through [`tr!`](crate::tools::tr) and [`tr_n!`](crate::tools::tr_n).
    ///
    /// Only available with the `register-translations` feature. The strings are gathered from the entire loaded GDExtension binary,
    /// independently of whether the surrounding code runs. The order is unspecified.
    ///
    /// See also [`write_pot()`] to directly produce a gettext `.pot` template.
    pub fn collect_translations() -> Vec<TranslationShard> {
        let mut result = Vec::new();
        crate::private::iterate_translation_shards(|shard| result.push(shard.clone()));
        result
    }

    /// Generates a gettext `.pot` template from all registered translation strings.
    ///
    /// Only available with the `register-translations` feature. Entries are deduplicated by `(context, msgid)`, with all source
    /// locations listed as `#:` references. The returned string can be written to a `.pot` file and fed into the standard gettext
    /// workflow (`msgmerge`, `.po`, Godot `Translation`).
    pub fn write_pot() -> String {
        write_pot_from(collect_translations())
    }

    /// Inner implementation of [`write_pot()`], decoupled from the global registry for testing.
    fn write_pot_from(shards: impl IntoIterator<Item = TranslationShard>) -> String {
        struct Entry {
            plural: Option<&'static str>,
            references: Vec<(&'static str, u32)>,
        }

        // BTreeMap for deterministic output, keyed by (context, msgid).
        let mut entries: BTreeMap<(Option<&'static str>, &'static str), Entry> = BTreeMap::new();

        for shard in shards {
            // The empty msgid is reserved for the PO header; emitting it as a regular entry would produce a duplicate, invalid `.pot`.
            if shard.msgid.is_empty() {
                continue;
            }

            let entry = entries
                .entry((shard.context, shard.msgid))
                .or_insert_with(|| Entry {
                    plural: None,
                    references: Vec::new(),
                });

            // Keep the first plural form encountered, if any. A `tr!` and a `tr_n!` sharing (context, msgid) thus merge into one
            // plural entry; this is a rare mismatch and gettext tooling handles it fine.
            if entry.plural.is_none() {
                entry.plural = shard.plural;
            }

            entry.references.push((shard.file, shard.line));
        }

        // Sort references so the `.pot` output is reproducible; registry order is unspecified (link order).
        for entry in entries.values_mut() {
            entry.references.sort_unstable();
        }

        let mut out = String::new();
        out.push_str("# Translation template generated by godot-rust.\n");
        out.push_str("msgid \"\"\n");
        out.push_str("msgstr \"\"\n");
        out.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
        // Placeholder, so that `msgfmt` accepts `msgstr[n]` entries; translators replace it per target language.
        out.push_str("\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n");

        for ((context, msgid), entry) in &entries {
            out.push('\n');

            for (file, line) in &entry.references {
                out.push_str("#: ");
                out.push_str(file);
                out.push(':');
                out.push_str(&line.to_string());
                out.push('\n');
            }

            if let Some(context) = context {
                out.push_str("msgctxt ");
                push_po_string(&mut out, context);
                out.push('\n');
            }

            out.push_str("msgid ");
            push_po_string(&mut out, msgid);
            out.push('\n');

            match entry.plural {
                Some(plural) => {
                    out.push_str("msgid_plural ");
                    push_po_string(&mut out, plural);
                    out.push('\n');
                    out.push_str("msgstr[0] \"\"\n");
                    out.push_str("msgstr[1] \"\"\n");
                }
                None => out.push_str("msgstr \"\"\n"),
            }
        }

        out
    }

    /// Appends `s` to `out` as a quoted, gettext-escaped PO string.
    fn push_po_string(out: &mut String, s: &str) {
        out.push('"');
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(c),
            }
        }
        out.push('"');
    }

    #[cfg(test)]
    mod tests {
        use super::{TranslationShard, collect_translations, write_pot_from};

        // `tr!`/`tr_n!` in a function that is never referenced anywhere. Self-registration relies on each invocation emitting a
        // `#[used]` static into `.init_array` (see `shard_execute_pre_main!`); this static is a free-standing item, so it must be
        // emitted and run at load time regardless of whether the enclosing function is ever called or codegen'd.
        #[allow(dead_code)]
        fn dead_translations() {
            let _ = crate::tr!("dead-code msgid");
            let _ = crate::tr!("animal"; "dead-code with context");
            let _ = crate::tr_n!(1; "one apple", "many apples");
            let _ = crate::tr_n!(1, "ui"; "one window", "many windows");

            // Non-literal context cannot be extracted statically, so it must register with `context = None`.
            let runtime_context = String::from("runtime");
            let _ = crate::tr!(runtime_context; "non-literal context msgid");
        }

        // Same guarantee for an uninstantiated generic: the registration item does not depend on `T`, so it is collected even though
        // the function is never monomorphized.
        #[allow(dead_code, clippy::extra_unused_type_parameters)] // `T` is deliberately unused: the test never instantiates this.
        fn dead_generic<T>() {
            let _ = crate::tr!("uninstantiated generic msgid");
        }

        #[test]
        fn self_registration_includes_dead_code() {
            // Deliberately do NOT reference `dead_translations` or `dead_generic` -- the whole point is that registration happens
            // for code that is never called, so the macros can be enumerated across the entire binary.
            let shards = collect_translations();

            assert!(shards.iter().any(|s| s.msgid == "dead-code msgid"
                && s.context.is_none()
                && s.plural.is_none()));
            assert!(
                shards
                    .iter()
                    .any(|s| s.msgid == "dead-code with context" && s.context == Some("animal"))
            );
            assert!(shards.iter().any(|s| s.msgid == "one apple"
                && s.plural == Some("many apples")
                && s.context.is_none()));
            assert!(shards.iter().any(|s| s.msgid == "one window"
                && s.plural == Some("many windows")
                && s.context == Some("ui")));
            assert!(
                shards
                    .iter()
                    .any(|s| s.msgid == "uninstantiated generic msgid")
            );
            assert!(
                shards
                    .iter()
                    .any(|s| s.msgid == "non-literal context msgid" && s.context.is_none())
            );
        }

        fn shard(
            msgid: &'static str,
            plural: Option<&'static str>,
            context: Option<&'static str>,
            line: u32,
        ) -> TranslationShard {
            TranslationShard::new(msgid, plural, context, "src/demo.rs", line)
        }

        #[test]
        fn pot_basic_entry() {
            let pot = write_pot_from([shard("Hello", None, None, 10)]);

            assert!(pot.contains("#: src/demo.rs:10\n"));
            assert!(pot.contains("msgid \"Hello\"\n"));
            assert!(pot.contains("msgstr \"\"\n"));
        }

        #[test]
        fn pot_context_and_plural() {
            let pot = write_pot_from([
                shard("apple", Some("apples"), Some("fruit"), 1),
                shard("Move", None, Some("verb"), 2),
            ]);

            assert!(pot.contains("msgctxt \"fruit\"\nmsgid \"apple\"\nmsgid_plural \"apples\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n"));
            assert!(pot.contains("msgctxt \"verb\"\nmsgid \"Move\"\nmsgstr \"\"\n"));
        }

        #[test]
        fn pot_dedup_merges_references() {
            // Same (context, msgid) from two locations -> one entry, two `#:` references.
            let pot = write_pot_from([
                shard("Hello", None, None, 10),
                shard("Hello", None, None, 42),
            ]);

            assert_eq!(pot.matches("msgid \"Hello\"\n").count(), 1);
            assert!(pot.contains("#: src/demo.rs:10\n#: src/demo.rs:42\n"));
        }

        #[test]
        fn pot_skips_empty_msgid() {
            // An empty msgid would collide with the reserved PO header entry, producing an invalid `.pot`; it must be skipped.
            let pot = write_pot_from([shard("", None, None, 1), shard("Hello", None, None, 2)]);

            assert_eq!(pot.matches("msgid \"\"\n").count(), 1); // Only the header.
            assert!(pot.contains("msgid \"Hello\"\n"));
        }

        #[test]
        fn pot_distinguishes_by_context() {
            // Same msgid but different context -> two distinct entries.
            let pot = write_pot_from([
                shard("Open", None, Some("file"), 1),
                shard("Open", None, Some("door"), 2),
            ]);

            assert_eq!(pot.matches("msgid \"Open\"\n").count(), 2);
        }

        #[test]
        fn pot_escapes_special_chars() {
            let pot = write_pot_from([shard("a\"b\\c\nd\te\rf", None, None, 1)]);

            assert!(pot.contains(r#"msgid "a\"b\\c\nd\te\rf""#));
        }
    }
}
