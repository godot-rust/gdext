/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Generates RID type marker enums for server-internal resource types.

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::{HashMap, HashSet};

use crate::conv::to_pascal_case;
use crate::special_cases::get_all_rid_method_prefixes;
use crate::util::ident;

/// Represents a generated RID tag.
struct RidTag {
    /// The Rust identifier for the tag (e.g., `CanvasTag` or `PhysicsServer2DSpaceTag`).
    name: Ident,
    /// The class this tag is associated with (for documentation).
    class_name: &'static str,
    /// The method prefix this tag corresponds to.
    method_prefix: &'static str,
}

/// Generates the RID tags module code.
pub fn make_rid_tags_code() -> TokenStream {
    let prefixes = get_all_rid_method_prefixes();

    // Count occurrences of each prefix to detect duplicates.
    let mut prefix_counts: HashMap<&str, usize> = HashMap::new();
    for (_, prefix) in prefixes {
        *prefix_counts.entry(*prefix).or_insert(0) += 1;
    }

    // Track which (class, prefix) pairs we've already processed to avoid duplicates.
    let mut seen: HashSet<(&str, &str)> = HashSet::new();

    // Generate tags.
    let mut tags: Vec<RidTag> = Vec::new();
    for (class_name, method_prefix) in prefixes {
        // Skip duplicates (e.g., NavigationServer2D and NavigationServer3D share same prefixes).
        if !seen.insert((class_name, method_prefix)) {
            continue;
        }

        let is_duplicate_prefix = prefix_counts.get(method_prefix).copied().unwrap_or(0) > 1;
        let tag_name = make_tag_name(class_name, method_prefix, is_duplicate_prefix);

        tags.push(RidTag {
            name: tag_name,
            class_name,
            method_prefix,
        });
    }

    // Generate the code for each tag.
    let tag_definitions: Vec<TokenStream> = tags
        .iter()
        .map(|tag| {
            let name = &tag.name;
            let class_name = tag.class_name;
            let method_prefix = tag.method_prefix;

            // Generate documentation linking to the class methods.
            let doc = format!(
                "Marker for RIDs returned by [`{class_name}`](crate::classes::{class_name}) methods with prefix `{method_prefix}*`."
            );

            quote! {
                #[doc = #doc]
                pub enum #name {}
                impl crate::meta::sealed::Sealed for #name {}
                impl crate::obj::TaggedRid for #name {}
            }
        })
        .collect();

    quote! {
        //! RID type markers for server-internal resource types.
        //!
        //! These marker types represent server-side resources that don't have corresponding
        //! Godot class types. They are used with [`TypedRid<T>`](crate::obj::TypedRid) to provide
        //! type safety for low-level server APIs.
        //!
        //! # Background
        //!
        //! Godot's server APIs ([`RenderingServer`](crate::classes::RenderingServer),
        //! [`PhysicsServer2D`](crate::classes::PhysicsServer2D),
        //! [`PhysicsServer3D`](crate::classes::PhysicsServer3D), etc.) work with RIDs that
        //! represent internal resources. While some RIDs correspond to scene tree classes
        //! (e.g., `Mesh`, `Shader`), many represent server-internal types that have no
        //! Godot class equivalent.
        //!
        //! These markers allow type-safe RID usage for those server-internal types.

        #( #tag_definitions )*
    }
}

/// Creates the tag name identifier based on the class and method prefix.
///
/// - If prefix is unique: `{MethodPrefix}Tag` (e.g., `canvas_` -> `CanvasTag`)
/// - If prefix is duplicate: `{Class}{MethodPrefix}Tag` (e.g., `space_` on `PhysicsServer2D` -> `PhysicsServer2DSpaceTag`)
fn make_tag_name(class_name: &str, method_prefix: &str, is_duplicate: bool) -> Ident {
    // Remove trailing underscore and convert to PascalCase.
    let prefix_without_underscore = method_prefix.trim_end_matches('_');
    let pascal_prefix = to_pascal_case(prefix_without_underscore);

    let name = if is_duplicate {
        format!("{class_name}{pascal_prefix}Tag")
    } else {
        format!("{pascal_prefix}Tag")
    };

    ident(&name)
}
