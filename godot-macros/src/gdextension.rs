/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::util::{bail, ident, validate_impl, KvParser};
use crate::ParseResult;

pub fn attribute_gdextension(item: venial::Item) -> ParseResult<TokenStream> {
    let mut impl_decl = match item {
        venial::Item::Impl(item) => item,
        _ => return bail!(&item, "#[gdextension] can only be applied to trait impls"),
    };

    validate_impl(&impl_decl, Some("ExtensionLibrary"), "gdextension")?;
    if impl_decl.tk_unsafe.is_none() {
        return bail!(
            impl_decl.tk_impl,
            "`impl ExtensionLibrary` must be marked unsafe, to confirm your opt-in to godot-rust's safety model",
        );
    }

    let drained_attributes = std::mem::take(&mut impl_decl.attributes);
    let mut parser = KvParser::parse_required(&drained_attributes, "gdextension", &impl_decl)?;
    let entry_point = parser.handle_ident("entry_point")?;
    let entry_symbol = parser.handle_ident("entry_symbol")?;
    parser.finish()?;

    if entry_point.is_some() && entry_symbol.is_some() {
        return bail!(
            impl_decl.tk_impl,
            "Cannot specify both `entry_point` and `entry_symbol` in #[gdextension] attribute",
        );
    }

    let deprecation = if entry_point.is_some() {
        quote! { ::godot::__deprecated::emit_deprecated_warning!(gdextension_entry_point); }
    } else {
        TokenStream::new()
    };

    let entry_point = entry_symbol
        .or(entry_point)
        .unwrap_or_else(|| ident("gdext_rust_init"));

    let impl_ty = &impl_decl.self_ty;

    Ok(quote! {
        #deprecation
        #impl_decl

        #[no_mangle]
        unsafe extern "C" fn #entry_point(
            get_proc_address: ::godot::sys::GDExtensionInterfaceGetProcAddress,
            library: ::godot::sys::GDExtensionClassLibraryPtr,
            init: *mut ::godot::sys::GDExtensionInitialization,
        ) -> ::godot::sys::GDExtensionBool {
            ::godot::init::__gdext_load_library::<#impl_ty>(
                get_proc_address,
                library,
                init
            )
        }

        fn __static_type_check() {
            // Ensures that the init function matches the signature advertised in FFI header
            let _unused: ::godot::sys::GDExtensionInitializationFunction = Some(#entry_point);
        }

        #[cfg(target_os = "linux")]
        ::godot::sys::register_hot_reload_workaround!();
    })
}
