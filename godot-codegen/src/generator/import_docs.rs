/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::context::Context;
use crate::models::domain::{ApiView, Class, ClassLike, Enum, Enumerator, Function, TyName};
use crate::{conv, special_cases, util};

type CowStr = std::borrow::Cow<'static, str>;

/// Infallible `write!`.
macro_rules! write_str {
    ($out:expr, $($arg:tt)*) => {
        write!($out, $($arg)*).expect("writing to String should not fail")
    };
}

pub fn import_docs(
    description: &str,
    surrounding_class: Option<&Class>,
    ctx: &Context,
    view: &ApiView,
) -> String {
    DocImporter::new(description, surrounding_class, ctx, view).import()
}

pub fn import_function_docs(fun: &dyn Function, ctx: &Context, view: &ApiView) -> Option<String> {
    let doc = fun.common().description.as_ref()?;
    if doc.is_empty() {
        return None;
    }
    let surrounding_class_name = fun.surrounding_class();
    let surrounding_class = surrounding_class_name.and_then(|name| view.find_engine_class(name));
    let imported_doc = import_docs(doc, surrounding_class, ctx, view);
    Some(imported_doc)
}

fn matches_primitive_type(ty: &str) -> bool {
    matches!(ty, "int" | "float" | "bool")
}

fn matches_ignored_links(class: &str) -> bool {
    // We don't have a single place to point @GDScript to.
    class == "@GDScript"
}

/// Flags controlling how a parse region is rendered.
#[derive(Copy, Clone)]
struct ParseMode {
    /// Convert single `\n` in source to `\n\n` (Markdown paragraph break).
    double_newlines: bool,
    /// Turn `[Type]` brackets into Rustdoc links; off inside code regions.
    allow_type_links: bool,
    /// Recurse into BBCode tags; off inside code regions where content is literal.
    allow_tags: bool,
}

impl ParseMode {
    /// Top-level prose: paragraph breaks, type links, BBCode all enabled.
    const TOP: Self = Self {
        double_newlines: true,
        allow_type_links: true,
        allow_tags: true,
    };
    /// Inside fenced/inline code: keep content literal, no paragraph breaks, no recursion.
    const CODE: Self = Self {
        double_newlines: false,
        allow_type_links: false,
        allow_tags: false,
    };

    /// Derive inner-content mode for a wrapped tag.
    ///
    /// Formatting tags (`[b]`, `[i]`, `[kbd]`) inherit the outer mode so nested links/tags work.
    /// Code tags switch to [`Self::CODE`] so brackets inside code stay literal.
    fn inherit_for(self, allow_inner: bool) -> Self {
        if allow_inner {
            Self {
                double_newlines: self.double_newlines,
                allow_type_links: self.allow_type_links,
                allow_tags: true,
            }
        } else {
            Self::CODE
        }
    }
}

/// BBCode tag with a fixed opener/closer.
///
/// Used for tags whose opener has no dynamic attribute. Tags with attributes
/// (`[url=...]`, `[codeblock lang=...]`) are handled by separate parsers.
enum WrappedTag {
    /// Tag and its content are emitted, wrapped in Markdown.
    Render {
        /// BBCode opener, e.g. `"[b]"`.
        open: &'static str,
        /// BBCode closer, e.g. `"[/b]"`.
        close: &'static str,
        /// Markdown to emit before inner content, e.g. `"**"` or `"```gdscript"`.
        prefix: &'static str,
        /// Markdown to emit after inner content, e.g. `"**"` or `"```"`.
        suffix: &'static str,
        /// If true, inner content is parsed with the outer mode (formatting tags like `[b]`, `[i]`, `[kbd]`).
        /// If false, inner content is parsed in [`ParseMode::CODE`] (fenced/inline code blocks).
        allow_inner: bool,
    },
    /// Tag and its content are dropped entirely. One trailing `\n` already in the output is also consumed,
    /// so the surrounding block doesn't gain a blank line.
    Skip {
        open: &'static str,
        close: &'static str,
    },
}

impl WrappedTag {
    fn open(&self) -> &'static str {
        match self {
            Self::Render { open, .. } | Self::Skip { open, .. } => open,
        }
    }
}

// Order matters: longer prefix first when prefixes overlap (`[code skip-lint]` before `[code]`).
#[rustfmt::skip]
const WRAPPED_TAGS: &[WrappedTag] = &[
    WrappedTag::Render { open: "[b]",              close: "[/b]",         prefix: "**",          suffix: "**",  allow_inner: true  },
    WrappedTag::Render { open: "[i]",              close: "[/i]",         prefix: "_",           suffix: "_",   allow_inner: true  },
    WrappedTag::Render { open: "[kbd]",            close: "[/kbd]",       prefix: "`",           suffix: "`",   allow_inner: true  },
    WrappedTag::Render { open: "[code skip-lint]", close: "[/code]",      prefix: "`",           suffix: "`",   allow_inner: false },
    WrappedTag::Render { open: "[code]",           close: "[/code]",      prefix: "`",           suffix: "`",   allow_inner: false },
    WrappedTag::Render { open: "[codeblock]",      close: "[/codeblock]", prefix: "```gdscript", suffix: "```", allow_inner: false },
    WrappedTag::Render { open: "[gdscript]",       close: "[/gdscript]",  prefix: "```gdscript", suffix: "```", allow_inner: false },
    // C# blocks usually just duplicate the adjacent `[gdscript]` block, and we have no C# audience in Rust docs.
    WrappedTag::Skip   { open: "[csharp]",         close: "[/csharp]" },
];

struct DocImporter<'d> {
    doc: &'d str,
    pos: usize,
    surrounding_class: Option<&'d Class>,
    ctx: &'d Context,
    view: &'d ApiView<'d>,
    /// Cache of Godot class name -> Rust crate path (e.g. `"Node"` -> `"crate::classes::Node"`).
    /// Lives for the duration of one doc-string import; avoids repeated `to_pascal_case` + `format!` per link.
    path_cache: HashMap<String, String>,
}

impl<'d> DocImporter<'d> {
    fn new(
        doc: &'d str,
        surrounding_class: Option<&'d Class>,
        ctx: &'d Context,
        view: &'d ApiView<'d>,
    ) -> Self {
        Self {
            doc,
            pos: 0,
            surrounding_class,
            ctx,
            view,
            path_cache: HashMap::new(),
        }
    }

    fn import(mut self) -> String {
        // Output grows ~3-4x when type links expand to `[`Foo`][crate::classes::Foo]`; reserve up front.
        let mut out = String::with_capacity(self.doc.len() * 4);
        let ok = self.parse_until(&mut out, None, ParseMode::TOP);
        debug_assert!(ok, "top-level parse_until without closing tag must succeed");
        out
    }

    /// Snapshot of `(self.pos, out.len())` for transactional rollback on failed sub-parses.
    fn checkpoint(&self, out: &str) -> (usize, usize) {
        (self.pos, out.len())
    }

    /// Restore both input position and output length from a [`Self::checkpoint`].
    /// Used when a sub-parser starts emitting then fails (e.g. unterminated tag) and must
    /// leave the source byte-for-byte for the fallback parser to consume.
    fn rollback(&mut self, out: &mut String, cp: (usize, usize)) {
        self.pos = cp.0;
        out.truncate(cp.1);
    }

    /// Advance `self.pos` past `target` without writing anything to output. Returns `true` if `target` was found.
    fn skip_past(&mut self, target: &str) -> bool {
        if let Some(offset) = self.doc[self.pos..].find(target) {
            self.pos += offset + target.len();
            true
        } else {
            false
        }
    }

    // Parses the doc, writing rendered output into `out`.
    // - If `closing_tag` is given, returns true when found and consumed.
    // - On EOF without closing tag, rolls back `out` and `self.pos` and returns false.
    // - With `closing_tag = None`, always succeeds at EOF.
    fn parse_until(
        &mut self,
        out: &mut String,
        closing_tag: Option<&str>,
        mode: ParseMode,
    ) -> bool {
        let cp = self.checkpoint(out);

        while self.pos < self.doc.len() {
            if let Some(close) = closing_tag
                && self.remaining().starts_with(close)
            {
                self.pos += close.len();
                return true;
            }

            if mode.allow_tags && self.remaining().starts_with('[') && self.try_parse_tag(out, mode)
            {
                continue;
            }

            // unwrap(): loop guard ensures self.pos < self.doc.len(), so a char is present.
            let ch = self.remaining().chars().next().unwrap();
            self.pos += ch.len_utf8();
            if ch == '\n' && mode.double_newlines {
                out.push_str("\n\n");
            } else {
                out.push(ch);
            }
        }

        if closing_tag.is_none() {
            true
        } else {
            self.rollback(out, cp);
            false
        }
    }

    /// Dispatch a `[...]` opener to the matching parser.
    ///
    /// Tries parsers in order:
    /// 1. Static [`WRAPPED_TAGS`] table.
    /// 2. Attribute-bearing tags: `[url=...]`, `[codeblocks]`, `[codeblock lang=...]`.
    /// 3. If a known BBCode opener is unterminated, return `false` -> the caller emits the bracket literally.
    /// 4. Generic Markdown link or type-link role.
    fn try_parse_tag(&mut self, out: &mut String, mode: ParseMode) -> bool {
        // Try the static table of wrapped tags first. An unterminated real opener is emitted as-is,
        // not reinterpreted by the bracket-link parser below.
        for tag in WRAPPED_TAGS {
            if self.try_wrapped_tag(out, tag, mode) {
                return true;
            }
        }

        if self.try_url_tag(out, mode)
            || self.try_codeblocks_tag(out)
            || self.try_codeblock_lang_tag(out)
        {
            return true;
        }

        if starts_with_known_tag(self.remaining()) {
            return false;
        }

        self.try_markdown_link(out) || self.try_bracket_link(out, mode.allow_type_links)
    }

    fn try_wrapped_tag(&mut self, out: &mut String, tag: &WrappedTag, mode: ParseMode) -> bool {
        if !self.remaining().starts_with(tag.open()) {
            return false;
        }

        let cp = self.checkpoint(out);
        self.pos += tag.open().len();

        match *tag {
            WrappedTag::Render {
                close,
                prefix,
                suffix,
                allow_inner,
                ..
            } => {
                out.push_str(prefix);
                if !self.parse_until(out, Some(close), mode.inherit_for(allow_inner)) {
                    self.rollback(out, cp);
                    return false;
                }
                out.push_str(suffix);
            }
            WrappedTag::Skip { close, .. } => {
                // Drop the tag and its body. On EOF without close, roll back to leave the opener literal.
                if !self.skip_past(close) {
                    self.rollback(out, cp);
                    return false;
                }

                // Consume one trailing `\n` already in `out`, so the dropped block doesn't leave a blank line behind it.
                if out.ends_with('\n') {
                    out.pop();
                }
            }
        }
        true
    }

    // Consume an opener of the form `<prefix>VALUE]`. Advances `self.pos` past the `]`.
    // Returns (start, end) byte offsets within `self.doc` instead of `&str`, so callers can keep parsing with `&mut self`
    // and reconstruct the slice later without allocating a temporary `String`.
    //
    // Shortcoming: searches for the first `]` in the remaining input, so an attribute value containing `]` (e.g. `[url=https://x/a]b]`)
    // would truncate. Godot's docs should not produce such values in practice, so this is accepted.
    fn try_consume_attr_opener(&mut self, prefix: &str) -> Option<(usize, usize)> {
        let remaining = self.remaining();
        if !remaining.starts_with(prefix) {
            return None;
        }
        let end = remaining.find(']')?;
        let value_start = self.pos + prefix.len();
        let value_end = self.pos + end;
        self.pos += end + 1;
        Some((value_start, value_end))
    }

    fn try_url_tag(&mut self, out: &mut String, mode: ParseMode) -> bool {
        const PREFIX: &str = "[url=";
        const SUFFIX: &str = "[/url]";

        let cp = self.checkpoint(out);
        let Some((url_start, url_end)) = self.try_consume_attr_opener(PREFIX) else {
            return false;
        };

        out.push('[');
        if !self.parse_until(out, Some(SUFFIX), mode) {
            self.rollback(out, cp);
            return false;
        }
        // Re-slice after parse_until: storing offsets avoids borrowing self.doc across the `&mut self` call and a `to_owned()`.
        write_str!(out, "]({})", &self.doc[url_start..url_end]);
        true
    }

    fn try_codeblocks_tag(&mut self, out: &mut String) -> bool {
        const OPENING_TAG: &str = "[codeblocks]";
        const CLOSING_TAG: &str = "[/codeblocks]";

        if !self.remaining().starts_with(OPENING_TAG) {
            return false;
        }

        let cp = self.checkpoint(out);
        self.pos += OPENING_TAG.len();
        // `[codeblocks]` is a container for nested language blocks, not a literal fence itself.
        let inner = ParseMode {
            double_newlines: false,
            allow_type_links: true,
            allow_tags: true,
        };
        if !self.parse_until(out, Some(CLOSING_TAG), inner) {
            self.rollback(out, cp);
            return false;
        }
        true
    }

    fn try_codeblock_lang_tag(&mut self, out: &mut String) -> bool {
        const PREFIX: &str = "[codeblock lang=";
        const SUFFIX: &str = "[/codeblock]";

        let cp = self.checkpoint(out);
        let Some((lang_start, lang_end)) = self.try_consume_attr_opener(PREFIX) else {
            return false;
        };

        // Write the fence opener first; storing offsets keeps the borrow local so parse_until can still take `&mut self`.
        {
            let lang = &self.doc[lang_start..lang_end];
            write_str!(out, "```{lang}");
        }
        // The body is literal fenced code, so bracket roles should not be interpreted inside it.
        if !self.parse_until(out, Some(SUFFIX), ParseMode::CODE) {
            self.rollback(out, cp);
            return false;
        }
        out.push_str("```");
        true
    }

    /// Pass-through for inline Markdown links `[text](http(s)://...)` already present in source.
    /// Bare `[Type](suffix)` (no `http`) is left for [`Self::try_bracket_link`] to handle.
    ///
    /// Shortcoming: scans for the first `]` and `)`, so nested brackets in link text (e.g. `[a [Node] b](http://x)`) would mis-parse.
    /// Not produced by Godot in practice.
    fn try_markdown_link(&mut self, out: &mut String) -> bool {
        let remaining = self.remaining();
        if !remaining.starts_with('[') {
            return false;
        }

        let Some(end_of_text) = remaining.find(']') else {
            return false;
        };
        let after_text = &remaining[end_of_text + 1..];
        // Preserve real inline Markdown links, but let `[Type](s)` fall back to type-link parsing.
        if !after_text.starts_with("(http") {
            return false;
        }

        let Some(end_of_target) = after_text.find(')') else {
            return false;
        };
        let len = end_of_text + 1 + end_of_target + 1;
        out.push_str(&remaining[..len]);
        self.pos += len;
        true
    }

    /// Handle role-prefixed brackets (`[param X]`, `[method Class.fn]`, `[signal X]`,
    /// `[annotation X]`, `[constructor Type]`), bare type links (`[Node]`), and
    /// "escaped" roles whose target we cannot resolve (`[member X.y]`, `[constant X]`, ...).
    /// Unrecognized brackets return `false` so the caller emits them literally.
    fn try_bracket_link(&mut self, out: &mut String, allow_type_links: bool) -> bool {
        let remaining = self.remaining();
        if !remaining.starts_with('[') {
            return false;
        }

        let Some(end) = remaining.find(']') else {
            return false;
        };
        let whole = &remaining[..=end];
        let content = &remaining[1..end];

        if let Some(param_name) = content.strip_prefix("param ")
            && is_ident_like(param_name)
        {
            self.pos += whole.len();
            write_str!(out, "`{param_name}`");
            return true;
        }

        if let Some(method_path) = content.strip_prefix("method ") {
            self.pos += whole.len();
            self.write_method_link(out, whole, method_path);
            return true;
        }

        if let Some(signal_name) = content.strip_prefix("signal ") {
            self.pos += whole.len();
            write_code_span(out, signal_name);
            return true;
        }

        if let Some(annotation) = content.strip_prefix("annotation ") {
            self.pos += whole.len();
            write_code_span(out, annotation);
            return true;
        }

        if let Some(constant) = content.strip_prefix("constant ") {
            self.pos += whole.len();
            self.write_constant_link(out, constant);
            return true;
        }

        if let Some(ty_name) = content.strip_prefix("constructor ") {
            self.pos += whole.len();
            out.push('`');
            out.push_str(ty_name);
            out.push_str("()`");
            return true;
        }

        if is_escaped_role(content) {
            self.pos += whole.len();
            write_str!(out, "\\{whole}");
            return true;
        }

        if allow_type_links && is_type_link(content) {
            self.pos += whole.len();
            self.write_type_link(out, content);
            return true;
        }

        false
    }

    /// Emit Markdown for `[Foo]`. Branches:
    /// - `@GDScript` -> plain text (no dedicated Rust module target);
    /// - deleted/disabled class -> `` `Foo` `` code span (class exists but has no Rust binding);
    /// - `int`/`float`/`bool` -> `` `int` `` (no link target);
    /// - hardcoded specials (`@GlobalScope`) -> fixed link;
    /// - link to surrounding class -> `` `Self` ``-style code span (avoid self-link);
    /// - else -> `` [`Foo`][crate::classes::Foo] `` Rustdoc reference link.
    fn write_type_link(&mut self, out: &mut String, ty_name: &str) {
        if matches_ignored_links(ty_name) {
            out.push_str(ty_name);
        } else if matches_primitive_type(ty_name) {
            write_code_span(out, ty_name);
        } else if let Some(hardcoded) = matches_hardcoded_type(ty_name) {
            out.push_str(hardcoded);
        } else if !self.ctx.is_builtin(ty_name) && special_cases::is_class_deleted_str(ty_name) {
            // Engine class excluded from codegen (or genuinely deleted): no link target. Builtins are always available, so skip the check.
            write_code_span(out, ty_name);
        } else if self
            .surrounding_class
            .is_some_and(|c| c.name().godot_ty == ty_name)
        {
            write_code_span(out, ty_name);
        } else {
            let path = self.cached_class_rust_path(ty_name);
            write_code_link(out, ty_name, path);
        }
    }

    /// Emit Markdown for `[method Class.fn]` or `[method fn]` (latter resolved against surrounding class).
    /// Falls back to escaping the original `[method ...]` literal if the target cannot be resolved.
    fn write_method_link(&mut self, out: &mut String, whole_match: &str, method_path: &str) {
        if let Some(method_path) = convert_to_method_path(
            method_path,
            self.surrounding_class,
            self.ctx,
            self.view,
            &mut self.path_cache,
        ) {
            let (_, method_name) = method_path
                .rsplit_once("::")
                .expect("rsplit_once should return a method name");
            write_str!(out, "[`{method_name}`][`{method_path}`]");
        } else {
            write_str!(out, "\\{whole_match}");
        }
    }

    /// Emit a link for `[constant X]` or `[constant ClassName.X]`.
    ///
    /// Lookup order for **bare** references (`[constant X]`, no dot):
    /// 1. Notification constants (`NOTIFICATION_*`) -- link to `crate::classes::notify::XyzNotification::X`, using the surrounding class's
    ///    notification enum when available, or the declaring class's otherwise.
    /// 2. Enum constants from the surrounding class's hierarchy (walks up via `base_class`) -- link to the sidecar module of the class in the
    ///    chain that declares the enum.
    /// 3. Global enum enumerators (e.g. `MIDI_MESSAGE_NOTE_ON`) -- link to `crate::global::MyEnum::XYZ`.
    /// 4. Fallback to a plain code span.
    ///
    /// For **class-scoped** references (`[constant ClassName.X]`, dot present):
    /// 1. Class enums (via [`Self::find_class_enum_constant`]).
    /// 2. Notification constants for the specified class.
    /// 3. Fallback to a plain code span.
    fn write_constant_link(&self, out: &mut String, godot_const_ref: &str) {
        if let Some((class_godot_name, const_godot_name)) = godot_const_ref.split_once('.') {
            // Class-scoped reference: e.g. "Node.NOTIFICATION_READY" or "Object.CONNECT_DEFERRED".
            if let Some((class, enum_, enumerator)) =
                self.find_class_enum_constant(class_godot_name, const_godot_name)
            {
                let module = format!("crate::classes::{}", class.mod_name().rust_mod);
                write_enum_const_link(out, &module, &enum_.name, &enumerator.name);
                return;
            }

            // Notification constant for the explicitly-specified class (e.g. "Node.NOTIFICATION_READY").
            if !class_godot_name.starts_with('@')
                && let Some((_decl_enum, variant)) =
                    self.ctx.find_notification_constant(const_godot_name)
            {
                let class_ty = TyName::from_godot(class_godot_name);
                if self.view.find_engine_class(&class_ty).is_some() {
                    let enum_name = self.ctx.notification_enum_name(&class_ty).name;
                    write_enum_const_link(out, "crate::classes::notify", &enum_name, &variant);
                    return;
                }
            }
        } else {
            // Global-scope reference -- try notifications, then class hierarchy, then global enums.

            // Notification constants (e.g. "NOTIFICATION_READY" -> "NodeNotification::READY").
            if let Some((decl_enum_ident, variant)) =
                self.ctx.find_notification_constant(godot_const_ref)
            {
                let enum_ident = self
                    .surrounding_class
                    .map(|c| self.ctx.notification_enum_name(c.name()).name)
                    .unwrap_or_else(|| decl_enum_ident.clone());
                write_enum_const_link(out, "crate::classes::notify", &enum_ident, &variant);
                return;
            }

            // Enum constant from the surrounding class or one of its base classes.
            if let Some(surrounding_class) = self.surrounding_class
                && let Some((class, enum_, enumerator)) =
                    self.find_class_enum_constant_in_hierarchy(surrounding_class, godot_const_ref)
            {
                let module = format!("crate::classes::{}", class.mod_name().rust_mod);
                write_enum_const_link(out, &module, &enum_.name, &enumerator.name);
                return;
            }

            // Global enum enumerator (e.g. "MIDI_MESSAGE_NOTE_ON").
            if let Some((enum_, enumerator)) = self.view.find_global_enum_constant(godot_const_ref)
            {
                let module = special_cases::get_global_enum_module_path(&enum_.godot_name);
                write_enum_const_link(out, module, &enum_.name, &enumerator.name);
                return;
            }
        }

        // Fallback: render as a code span when the constant cannot be resolved.
        write_code_span(out, godot_const_ref);
    }

    /// Look up a class-scoped enum enumerator by the Godot class and enumerator names.
    ///
    /// Finds the class in O(1) via [`ApiView`], then searches within that class's enums. The
    /// search within a single class is bounded in practice (classes typically have few enums).
    fn find_class_enum_constant(
        &self,
        class_godot_name: &str,
        const_godot_name: &str,
    ) -> Option<(&'d Class, &'d Enum, &'d Enumerator)> {
        // Names like `@GlobalScope` are not valid Rust identifiers and are not engine classes.
        if class_godot_name.starts_with('@') {
            return None;
        }
        let class_ty = TyName::from_godot(class_godot_name);
        let class = self.view.find_engine_class(&class_ty)?;
        let (enum_, enumerator) = self
            .view
            .find_class_enum_constant(&class_ty, const_godot_name)?;
        Some((class, enum_, enumerator))
    }

    /// Walk from `starting_class` up the inheritance chain, searching each class's enums for an
    /// enumerator matching `const_godot_name`.
    ///
    /// Returns the first match as `(declaring_class, enum, enumerator)`, or `None`.
    fn find_class_enum_constant_in_hierarchy(
        &self,
        starting_class: &'d Class,
        const_godot_name: &str,
    ) -> Option<(&'d Class, &'d Enum, &'d Enumerator)> {
        let mut current = starting_class;
        loop {
            if let Some((enum_, enumerator)) = self
                .view
                .find_class_enum_constant(current.name(), const_godot_name)
            {
                return Some((current, enum_, enumerator));
            }
            let base_name = current.base_class.as_ref()?;
            current = self.view.find_engine_class(base_name)?;
        }
    }

    /// Return the Rust crate path for a Godot class name, caching the result for the lifetime of this import.
    fn cached_class_rust_path(&mut self, godot_class_name: &str) -> &str {
        if !self.path_cache.contains_key(godot_class_name) {
            let path = get_class_rust_path(godot_class_name, self.ctx).into_owned();
            self.path_cache.insert(godot_class_name.to_owned(), path);
        }
        &self.path_cache[godot_class_name]
    }

    fn remaining(&self) -> &'d str {
        &self.doc[self.pos..]
    }
}

/// Emit `` `text` ``.
fn write_code_span(out: &mut String, text: &str) {
    write_str!(out, "`{text}`");
}

/// Emit a Rustdoc reference link `` [`label`][path] ``.
fn write_code_link(out: &mut String, label: &str, path: &str) {
    write_str!(out, "[`{label}`][{path}]");
}

/// Emit `` [`Enum::Variant`][`module::Enum::Variant`] ``, the form used for class- and global-enum links.
fn write_enum_const_link(
    out: &mut String,
    module_path: &str,
    enum_name: &dyn std::fmt::Display,
    variant: &dyn std::fmt::Display,
) {
    write_str!(
        out,
        "[`{enum_name}::{variant}`][`{module_path}::{enum_name}::{variant}`]"
    );
}

/// Bracketed name acceptable as a parameter/identifier (ASCII alphanumeric + underscore).
fn is_ident_like(str: &str) -> bool {
    !str.is_empty()
        && str
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Bracketed name shaped like a Godot class link: ASCII alphanumeric, optionally prefixed with `@`
/// for global namespaces (`@GlobalScope`, `@GDScript`). `@` only allowed at position 0.
fn is_type_link(str: &str) -> bool {
    let mut chars = str.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() && first != '@' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric())
}

/// True if `str` begins with a recognized BBCode opener. Used by [`DocImporter::try_parse_tag`]
/// to emit unterminated known openers as-is, instead of letting the bracket-link parser reinterpret them.
fn starts_with_known_tag(str: &str) -> bool {
    WRAPPED_TAGS.iter().any(|t| str.starts_with(t.open()))
        || str.starts_with("[url=")
        || str.starts_with("[codeblocks]")
        || str.starts_with("[codeblock lang=")
}

/// Roles we recognize but cannot resolve to a Rust target -- emit them as escaped literal `\[role X]`
/// so Markdown does not interpret them as links. Accepts both `[role arg]` and bare `[role]`.
fn is_escaped_role(str: &str) -> bool {
    let role = str.split_once(' ').map(|(r, _)| r).unwrap_or(str);
    matches!(
        role,
        "constant" | "member" | "enum" | "signal" | "annotation" | "constructor"
    )
}

fn convert_to_method_path(
    class_method: &str,
    surrounding_class: Option<&Class>,
    ctx: &Context,
    view: &ApiView,
    path_cache: &mut HashMap<String, String>,
) -> Option<CowStr> {
    // Get the class name from the link if it has one, otherwise use the surrounding class's name.
    // For example, in `CanvasItem` docs the link `[method Object.notification]` is owned by `Object`, while bare `[method queue_redraw]`
    // resolves against the surrounding `CanvasItem`.
    let (link_godot_class, link_godot_method) =
        if let Some((class_name, method_name)) = class_method.split_once('.') {
            (class_name, method_name)
        } else {
            let class = surrounding_class?;
            (class.name().godot_ty.as_str(), class_method)
        };

    let link_godot_method = util::safe_ident(link_godot_method).to_string();

    // These cover renamed helpers and special symbols that do not map 1:1 through the API view.
    // Run before the deletion check, so hardcoded mappings (e.g. `@GlobalScope.*`) keep working.
    match matches_hardcoded_method(link_godot_class, &link_godot_method) {
        Hardcoded::Mapped(path) => return Some(path),
        Hardcoded::Suppressed => return None,
        Hardcoded::NotMatched => {}
    }

    // Skip for builtins (Vector2, Transform2D, ...): they are always available and the exclusion list doesn't apply to them.
    if !ctx.is_builtin(link_godot_class) && special_cases::is_class_deleted_str(link_godot_class) {
        return None;
    }

    let rust_class_path: &str = {
        if !path_cache.contains_key(link_godot_class) {
            let path = get_class_rust_path(link_godot_class, ctx).into_owned();
            path_cache.insert(link_godot_class.to_owned(), path);
        }
        &path_cache[link_godot_class]
    };

    let Some(class) = view.find_engine_class(&TyName::from_godot(link_godot_class)) else {
        // Builtins (Vector2, Transform2D, ...) are not engine classes in the API view; their methods aren't captured here, so we trust the link.
        return Some(format!("{rust_class_path}::{link_godot_method}").into());
    };

    let Some(method) = class
        .methods
        .iter()
        .find(|method| method.godot_name() == link_godot_method)
    else {
        // Class is in the API view but the method isn't (excluded from default codegen, or shadowed by an `_ex` builder).
        // Fabricating a path would yield a broken link.
        return None;
    };

    // Type-safe replacements (e.g. `Object.get_script`): the codegen-generated method has a `raw_` prefix and is `pub(crate)`;
    // the public Rust replacement keeps the original Godot name. Link to the public replacement.
    if special_cases::is_class_method_replaced_with_type_safe(class.name(), &link_godot_method) {
        return Some(format!("{rust_class_path}::{link_godot_method}").into());
    }

    if method.is_private_in_final_api() {
        return None;
    }

    if method.is_virtual() {
        // Final classes don't have an associated trait with virtual methods.
        return (!class.is_final).then(|| {
            format!(
                "crate::classes::{}::{}",
                class.name().virtual_trait_name(),
                method.name()
            )
            .into()
        });
    }

    // Use the Rust name; covers `special_cases::maybe_rename_class_method`. Example: `GDScript.new` -> `instantiate`.
    Some(format!("{rust_class_path}::{}", method.name()).into())
}

fn matches_hardcoded_type(godot_class: &str) -> Option<&'static str> {
    match godot_class {
        "@GlobalScope" => Some("[@GlobalScope][crate::global]"),
        _ => None,
    }
}

enum Hardcoded {
    /// Matched a special-cased mapping; use this Rust path.
    Mapped(CowStr),
    /// Matched, but link should be dropped (no Rust target -- e.g. arbitrary `@GDScript` symbols).
    Suppressed,
    /// No special case; fall through to the regular API-view lookup.
    NotMatched,
}

fn matches_hardcoded_method(godot_class: &str, godot_method: &str) -> Hardcoded {
    let path: CowStr = match (godot_class, godot_method) {
        ("Object", "free") => "crate::obj::Gd::free".into(),
        ("Object", "get_instance_id") => "crate::obj::Gd::instance_id".into(),
        ("Object", "notification") => "crate::classes::Object::notify".into(),
        ("Object", "_notification") => "crate::classes::IObject::on_notification".into(),
        ("Object", "_init") => "crate::classes::IObject::init".into(),
        ("Object", "_validate_property") => "crate::classes::IObject::on_validate_property".into(),
        ("Object", "_get_property_list") => "crate::classes::IObject::on_get_property_list".into(),
        ("Object", "_get") => "crate::classes::IObject::on_get".into(),
        ("Object", "_set") => "crate::classes::IObject::on_set".into(),
        ("GDScript", "new") => "crate::obj::NewGd::new_gd".into(),
        ("String", "length") => "crate::builtin::GString::len".into(),
        ("String", "match_") => "crate::builtin::GString::match_glob".into(),
        ("Dictionary", "size") => "crate::builtin::Dictionary::len".into(),
        ("Array", "size") => "crate::builtin::AnyArray::len".into(),
        ("PackedByteArray", "size") => "crate::builtin::PackedByteArray::len".into(),
        ("Vector2", "min") => "crate::builtin::Vector2::coord_min".into(),
        ("Vector2", "max") => "crate::builtin::Vector2::coord_max".into(),
        ("Vector3", "min") => "crate::builtin::Vector3::coord_min".into(),
        ("Vector3", "max") => "crate::builtin::Vector3::coord_max".into(),
        ("Vector4", "min") => "crate::builtin::Vector4::coord_min".into(),
        ("Vector4", "max") => "crate::builtin::Vector4::coord_max".into(),
        ("Transform2D", "get_scale") => "crate::builtin::Transform2D::scale".into(),
        ("Node", "get_node") => "crate::classes::Node::get_node_as".into(),
        ("Color", "is_equal_approx") => "crate::builtin::math::ApproxEq::approx_eq".into(),
        ("@GlobalScope", "instance_from_id") => "crate::obj::Gd::from_instance_id".into(),
        ("@GlobalScope", "is_instance_valid") => "crate::obj::Gd::is_instance_valid".into(),
        ("@GDScript", "load") => "crate::tools::load".into(),
        ("@GDScript", "save") => "crate::tools::save".into(),
        ("@GlobalScope", _) => format!("crate::global::{godot_method}").into(),
        ("@GDScript", _) => return Hardcoded::Suppressed,
        _ => return Hardcoded::NotMatched,
    };

    Hardcoded::Mapped(path)
}

fn convert_builtin_types(type_name: &str) -> Option<&'static str> {
    match type_name {
        "String" => Some("crate::builtin::GString"),
        "Array" => Some("crate::builtin::Array"),
        "Dictionary" => Some("crate::builtin::Dictionary"),
        _ => None,
    }
}

fn get_class_rust_path(godot_class_name: &str, ctx: &Context) -> CowStr {
    if let Some(hardcoded_builtin_type) = convert_builtin_types(godot_class_name) {
        return hardcoded_builtin_type.into();
    }

    let is_builtin = ctx.is_builtin(godot_class_name);
    let rust_class_name = conv::to_pascal_case(godot_class_name);
    if is_builtin {
        format!("crate::builtin::{rust_class_name}").into()
    } else {
        format!("crate::classes::{rust_class_name}").into()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Unit tests

#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
#[allow(non_snake_case)]
mod tests {
    use std::cell::OnceCell;

    use super::*;
    use crate::models::api_json::load_extension_api;
    use crate::models::domain::ExtensionApi;

    // `Context` and `ExtensionApi` are cached per thread; `ApiView` is cheap to rebuild per test.
    // Using `thread_local` (rather than a global `OnceLock`) avoids needing a `Mutex`, since
    // `ExtensionApi` contains `proc_macro2::TokenStream` and is therefore `!Sync`.
    struct DocTestCache {
        ctx: Context,
        api: ExtensionApi,
    }

    thread_local! {
        static CACHE: OnceCell<DocTestCache> = const { OnceCell::new() };
    }

    fn import_doc_for_test(description: &str, surrounding_class_name: Option<&str>) -> String {
        CACHE.with(|cell| {
            let cache = cell.get_or_init(|| {
                let mut watch = godot_bindings::StopWatch::start();
                let json = load_extension_api(&mut watch);
                let mut ctx = Context::build_from_api(&json);
                let api = ExtensionApi::from_json(json, &mut ctx);
                DocTestCache { ctx, api }
            });

            let view = ApiView::new(&cache.api);
            let surrounding_class = surrounding_class_name
                .and_then(|name| view.find_engine_class(&TyName::from_godot(name)));

            import_docs(description, surrounding_class, &cache.ctx, &view)
        })
    }

    // Bare Godot type links become Rustdoc links with code-formatted labels.
    // Uses classes always present in default codegen (`Node`, `Resource`); excluded classes resolve to a code span instead.
    #[test]
    fn type__engine_classes() {
        let description = "Inherits from [Node] or [Resource], depending on use case.";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "Inherits from [`Node`][crate::classes::Node] or [`Resource`][crate::classes::Resource], depending on use case."
        );
    }

    #[test]
    fn type__builtin_and_member_role() {
        let description = "Link [member Vector2.x] and [member Vector2.y] on [Vector2] or \
            [Vector3]. Use [code]\"suffix:px/s\"[/code] for the editor unit suffix.";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "Link \\[member Vector2.x] and \\[member Vector2.y] on \
            [`Vector2`][crate::builtin::Vector2] or [`Vector3`][crate::builtin::Vector3]. Use \
            `\"suffix:px/s\"` for the editor unit suffix."
        );
    }

    // Existing Markdown links must stay untouched while later bare type links are still imported.
    #[test]
    fn type__preserves_markdown_link() {
        let description = "See [reference](https://example.com) and [Node].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "See [reference](https://example.com) and [`Node`][crate::classes::Node]."
        );
    }

    #[test]
    fn type__global_scope() {
        let description = "Use [@GlobalScope].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Use [@GlobalScope][crate::global].");
    }

    // Bare `@GDScript` stays plain until there is a dedicated Rust target for it.
    #[test]
    fn type__gdscript_plain_text() {
        let description = "See [@GDScript].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "See @GDScript.");
    }

    // Deleted/disabled classes (e.g. Android-only) become a backtick code span, not a broken link.
    #[test]
    #[cfg(not(target_os = "android"))] #[cfg_attr(published_docs, doc(cfg(not(target_os = "android"))))]
    fn type__deleted_class_backtick() {
        let description = "See [JavaClass].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "See `JavaClass`.");
    }

    #[test]
    fn type__primitive_links() {
        let description = "Use [int], [float], and [bool].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Use `int`, `float`, and `bool`.");
    }

    // Links to the current class are rendered as code to avoid redundant Markdown links.
    #[test]
    fn type__surrounding_class() {
        let description = "See [Node] and [Object].";

        let actual = import_doc_for_test(description, Some("Node"));

        assert_eq!(actual, "See `Node` and [`Object`][crate::classes::Object].");
    }

    #[test]
    fn method__with_newlines_and_roles() {
        let description = "Compare [code]LEFT[/code] and [code]RIGHT[/code] variants.\n\
            Use [method InputEvent.is_match] with [constant KEY_LOCATION_UNSPECIFIED] or [enum KeyLocation].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "Compare `LEFT` and `RIGHT` variants.\n\n\
            Use [`is_match`][`crate::classes::InputEvent::is_match`] with \
            [`KeyLocation::UNSPECIFIED`][`crate::global::KeyLocation::UNSPECIFIED`] or \\[enum KeyLocation]."
        );
    }

    #[test]
    fn method__in_surrounding_class() {
        let description = "Call [method get_node] to fetch a child.";

        let actual = import_doc_for_test(description, Some("Node"));

        assert_eq!(
            actual,
            "Call [`get_node_as`][`crate::classes::Node::get_node_as`] to fetch a child."
        );
    }

    // Type-safe replacements link to the public Rust method (which keeps the original Godot name), not the `pub(crate) raw_*` codegen variant.
    #[test]
    fn method__type_safe_replacement() {
        let description = "See [method Object.get_script].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "See [`get_script`][`crate::classes::Object::get_script`]."
        );
    }

    // `[constant X]` falls back to a code span when the name doesn't resolve to any known constant.
    // Here, `CONNECT_DEFERRED` is a class-scoped constant that requires surrounding-class context,
    // which is absent, so it cannot be resolved.
    #[test]
    fn constant__code_fallback() {
        let description = "See [constant CONNECT_DEFERRED].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "See `CONNECT_DEFERRED`.");
    }

    // `[constant NOTIFICATION_X]` without a surrounding class links to the declaring class's enum.
    #[test]
    fn constant__notification_link_no_class() {
        let description = "See [constant NOTIFICATION_ENTER_TREE].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "See [`NodeNotification::ENTER_TREE`][`crate::classes::notify::NodeNotification::ENTER_TREE`]."
        );
    }

    // `[constant NOTIFICATION_X]` with a surrounding class links to that class's notification enum.
    #[test]
    fn constant__notification_link_with_class() {
        let description = "See [constant NOTIFICATION_READY].";

        let actual = import_doc_for_test(description, Some("Node"));

        assert_eq!(
            actual,
            "See [`NodeNotification::READY`][`crate::classes::notify::NodeNotification::READY`]."
        );
    }

    // An inherited notification constant uses the surrounding class's (not declaring class's) enum.
    // `NOTIFICATION_POSTINITIALIZE` is declared by `Object`, but in `Node` docs it links to `NodeNotification`.
    #[test]
    fn constant__notification_link_inherited() {
        let description = "See [constant NOTIFICATION_POSTINITIALIZE].";

        let actual = import_doc_for_test(description, Some("Node"));

        assert_eq!(
            actual,
            "See [`NodeNotification::POSTINITIALIZE`][`crate::classes::notify::NodeNotification::POSTINITIALIZE`]."
        );
    }

    // A class enum constant referenced without a dot is resolved via the surrounding class hierarchy.
    // `CONNECT_DEFERRED` is in `Object::ConnectFlags`; accessed from a `Node`-surrounding context it
    // walks up to `Object` and links to the sidecar module.
    #[test]
    fn constant__class_enum_via_hierarchy() {
        let description = "See [constant CONNECT_DEFERRED].";

        let actual = import_doc_for_test(description, Some("Node"));

        assert_eq!(
            actual,
            "See [`ConnectFlags::DEFERRED`][`crate::classes::object::ConnectFlags::DEFERRED`]."
        );
    }

    // `[constant ClassName.X]` with a dot resolves via the named class's sidecar enum.
    #[test]
    fn constant__dot_class_enum_link() {
        let description = "See [constant Object.CONNECT_DEFERRED].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "See [`ConnectFlags::DEFERRED`][`crate::classes::object::ConnectFlags::DEFERRED`]."
        );
    }

    // `[constant ClassName.NOTIFICATION_X]` with a dot resolves to that class's notification enum.
    #[test]
    fn constant__dot_notification_link() {
        let description = "See [constant Node.NOTIFICATION_READY].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "See [`NodeNotification::READY`][`crate::classes::notify::NodeNotification::READY`]."
        );
    }

    // `[constant X]` for a global enum enumerator emits a Rustdoc link to the Rust variant.
    #[test]
    fn constant__global_enum_link() {
        let description = "See [constant MIDI_MESSAGE_NOTE_ON].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "See [`MidiMessage::NOTE_ON`][`crate::global::MidiMessage::NOTE_ON`]."
        );
    }

    #[test]
    fn code_block__preserves_contents() {
        let description = "Bit mask used to remove modifiers before checking a keycode.\n\
            [codeblock]\n\
            var keycode = KEY_A | KEY_MASK_SHIFT\n\
            keycode = keycode & KEY_CODE_MASK\n\
            [/codeblock]";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "Bit mask used to remove modifiers before checking a keycode.\n\n\
            ```gdscript\n\
            var keycode = KEY_A | KEY_MASK_SHIFT\n\
            keycode = keycode & KEY_CODE_MASK\n\
            ```"
        );
    }

    // Fenced code blocks keep bracketed type-like text literal.
    #[test]
    fn code_block__type_literal() {
        let description = "[codeblock]\n[Node]\n[/codeblock]";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "```gdscript\n[Node]\n```");
    }

    // Covers [codeblocks], [codeblock lang=...], [gdscript], and [csharp].
    #[test]
    fn code_block__other_variants() {
        let codeblocks_description = "[codeblocks]alpha\nbeta[/codeblocks]";
        let codeblock_lang_description = "[codeblock lang=text]\nalpha\nbeta\n[/codeblock]";
        let gdscript_description = "[gdscript]\nprint(\"hi\")\n[/gdscript]";
        let csharp_description = "[csharp]\nGD.Print(\"hi\");\n[/csharp]";

        let codeblocks_actual = import_doc_for_test(codeblocks_description, None);
        let codeblock_lang_actual = import_doc_for_test(codeblock_lang_description, None);
        let gdscript_actual = import_doc_for_test(gdscript_description, None);
        let csharp_actual = import_doc_for_test(csharp_description, None);

        assert_eq!(codeblocks_actual, "alpha\nbeta");
        assert_eq!(codeblock_lang_actual, "```text\nalpha\nbeta\n```");
        assert_eq!(gdscript_actual, "```gdscript\nprint(\"hi\")\n```");
        assert_eq!(csharp_actual, ""); // C# is currently hidden from Rust docs; it often just duplicates GDScript.
        // assert_eq!(csharp_actual, "```csharp\nGD.Print(\"hi\");\n```");
    }

    #[test]
    fn codeblocks__nested_languages() {
        let description = "[codeblocks]\n\
            [gdscript]\n\
            print(\"hi\")\n\
            [/gdscript]\n\
            [csharp]\n\
            GD.Print(\"hi\");\n\
            [/csharp]\n\
            [/codeblocks]";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "\n\
            ```gdscript\n\
            print(\"hi\")\n\
            ```\n" // C# is currently hidden from Rust docs; it often just duplicates GDScript.
        );
    }

    #[test]
    fn code__skip_lint() {
        let description =
            "Use [code skip-lint]x[/code] and [code skip-lint][url=address]text[/url][/code].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Use `x` and `[url=address]text[/url]`.");
    }

    // Inline code keeps bracket roles literal.
    #[test]
    fn code__member_literal() {
        let description = "Literal [code][member Vector2.x][/code].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Literal `[member Vector2.x]`.");
    }

    // Inline code spans keep bracketed type-like text literal.
    #[test]
    fn code__type_literal() {
        let description = "Literal [code][Node][/code].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Literal `[Node]`.");
    }

    // Inline code keeps method roles literal.
    #[test]
    fn code__method_literal() {
        let description = "Literal [code][method lerp][/code].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Literal `[method lerp]`.");
    }

    #[test]
    fn bold__with_code_and_member_role() {
        let description = "MIDI note release.\n\
            [b]Note:[/b] Some devices send [constant MIDI_MESSAGE_NOTE_ON] with [member InputEventMIDI.velocity] = [code]0[/code].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "MIDI note release.\n\n\
            **Note:** Some devices send [`MidiMessage::NOTE_ON`][`crate::global::MidiMessage::NOTE_ON`] with \
            \\[member InputEventMIDI.velocity] = `0`."
        );
    }

    #[test]
    fn url__basic() {
        let description = "Controller docs vary; see \
            [url=https://example.com/spec]the spec[/url] for sliders, pedals, and similar inputs.";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "Controller docs vary; see [the spec](https://example.com/spec) for sliders, \
            pedals, and similar inputs."
        );
    }

    #[test]
    fn italic__basic() {
        let description = "The current instrument is often called [i]program[/i] or \
            [i]preset[/i] in MIDI docs.";

        let actual = import_doc_for_test(description, None);

        assert_eq!(
            actual,
            "The current instrument is often called _program_ or _preset_ in MIDI docs."
        );
    }

    #[test]
    fn param__nested_in_bold() {
        let description = "[b]Use [param count][/b].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "**Use `count`**.");
    }

    #[test]
    fn kbd__basic() {
        let description = "Press [kbd]Ctrl + S[/kbd].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Press `Ctrl + S`.");
    }

    // Signal roles fall back to code-formatted text.
    #[test]
    fn signal__code_fallback() {
        let description = "Emit [signal pressed].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Emit `pressed`.");
    }

    // Annotation roles fall back to code-formatted text.
    #[test]
    fn annotation__code_fallback() {
        let description = "Use [annotation @GDScript.@rpc].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Use `@GDScript.@rpc`.");
    }

    // Constructor roles fall back to code-formatted text.
    #[test]
    fn constructor__code_fallback() {
        let description = "Create [constructor Transform2D].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Create `Transform2D()`.");
    }

    // A type-like bracket directly followed by `(http...)` must stay a Markdown link, not a Rust doc link.
    #[test]
    fn type__followed_by_http_url() {
        let description = "See [Node](https://example.com) for details.";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "See [Node](https://example.com) for details.");
    }

    #[test]
    fn type__followed_by_plural_suffix() {
        let description = "Use [Node](s).";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Use [`Node`][crate::classes::Node](s).");
    }

    // Unterminated BBCode stays literal instead of falling through into type-link parsing.
    #[test]
    fn bbcode__unclosed_tag_literal() {
        let description = "[b]hello";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "[b]hello");
    }

    // Inline code keeps nested BBCode literal.
    #[test]
    fn code__nested_bold() {
        let description = "Use [code][b]x[/b][/code].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Use `[b]x[/b]`.");
    }

    // Empty brackets `[]` are passed through untouched.
    #[test]
    fn empty_brackets() {
        let description = "Edge case: [].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Edge case: [].");
    }

    // A `[param ...]` whose name is not an identifier, such as `foo-bar`, is left untouched.
    #[test]
    fn param__non_ident_left_literal() {
        let description = "Bad name [param foo-bar].";

        let actual = import_doc_for_test(description, None);

        assert_eq!(actual, "Bad name [param foo-bar].");
    }
}
