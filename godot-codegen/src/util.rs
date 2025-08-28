/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with godot-macros crate.

use proc_macro2::{Ident, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote};

use crate::models::domain::ClassCodegenLevel;
use crate::models::json::JsonClass;
use crate::special_cases;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Small utility that turns an optional vector (often encountered as JSON deserialization type) into a slice.
pub fn option_as_slice<T>(option: &Option<Vec<T>>) -> &[T] {
    option.as_ref().map_or(&[], Vec::as_slice)
}

pub fn make_imports() -> TokenStream {
    quote! {
        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::meta::{AsArg, AsObjectArg, ClassName, CowArg, InParamTuple, ObjectArg, ObjectCow, OutParamTuple, ParamTuple, RefArg, Signature};
        use crate::classes::native::*;
        use crate::classes::Object;
        use crate::obj::Gd;
        use crate::sys::GodotFfi as _;
    }
}

pub fn c_str(string: &str) -> Literal {
    let c_string = std::ffi::CString::new(string).expect("CString::new() failed");
    Literal::c_string(&c_string)
}

pub fn make_string_name(identifier: &str) -> TokenStream {
    let lit = c_str(identifier);

    quote! { StringName::from(#lit) }
}

pub fn make_sname_ptr(identifier: &str) -> TokenStream {
    quote! {
        string_names.fetch(#identifier)
    }
}

pub fn get_api_level(class: &JsonClass) -> ClassCodegenLevel {
    // NOTE: We have to use a whitelist of known classes because Godot doesn't separate these out
    // beyond "editor" and "core" and some classes are also  mis-classified in the JSON depending on the Godot version.
    if let Some(forced_classification) =
        special_cases::classify_codegen_level(&class.name, &class.api_type)
    {
        return forced_classification;
    }

    // Fall back to just trusting the categorization. Presently, Godot only reports "core" and "editor".
    for level in ClassCodegenLevel::with_tables() {
        if level.lower() == class.api_type {
            return level;
        }
    }
    panic!(
        "class {} has unknown API type {}",
        class.name, class.api_type
    );
}

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn lifetime(s: &str) -> TokenStream {
    let tk_apostrophe = TokenTree::Punct(Punct::new('\'', Spacing::Joint));
    let tk_lifetime = TokenTree::Ident(ident(s));

    TokenStream::from_iter([tk_apostrophe, tk_lifetime])
}

// This function is duplicated in godot-macros\src\util\mod.rs
#[rustfmt::skip]
pub fn safe_ident(s: &str) -> Ident {
    // See also: https://doc.rust-lang.org/reference/keywords.html
    match s {
        // Lexer
        | "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
        | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"

        // Lexer 2018+
        | "async" | "await" | "dyn"

        // Reserved
        | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"

        // Reserved 2018+
        | "try"
           => format_ident!("{}_", s),

         _ => ident(s)
    }
}
