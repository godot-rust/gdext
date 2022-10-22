/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{bail, ident, parse_kv_group, path_is_single, validate_impl, KvValue};
use proc_macro2::TokenStream;
use quote::quote;
use venial::Declaration;

pub fn transform(meta: TokenStream, input: TokenStream) -> Result<TokenStream, venial::Error> {
    // Hack because venial doesn't support direct meta parsing yet
    let input = quote! {
        #[gdextension(#meta)]
        #input
    };

    let decl = venial::parse_declaration(input)?;

    let mut impl_decl = match decl {
        Declaration::Impl(item) => item,
        _ => return bail("#[gdextension] can only be applied to trait impls", &decl),
    };

    validate_impl(&impl_decl, Some("ExtensionLibrary"), "gdextension")?;
    if impl_decl.tk_unsafe.is_none() {
        return bail(
            "`impl ExtensionLibrary` must be marked unsafe, to confirm your opt-in to godot-rust's safety model", 
            impl_decl.tk_impl
        );
    }

    let mut entry_point = None;
    for attr in impl_decl.attributes.drain(..) {
        if path_is_single(&attr.path, "gdextension") {
            for (k, v) in parse_kv_group(&attr.value).expect("#[gdextension] has invalid arguments")
            {
                match (k.as_str(), v) {
                    ("entry_point", KvValue::Ident(f)) => entry_point = Some(f),
                    _ => return bail(&format!("#[gdextension]: invalid argument `{k}`"), attr),
                }
            }
        }
    }

    let entry_point = entry_point.unwrap_or(ident("gdextension_rust_init"));
    let impl_ty = &impl_decl.self_ty;

    Ok(quote! {
        #impl_decl

        #[no_mangle]
        unsafe extern "C" fn #entry_point(
            interface: *const ::godot_ffi::GDNativeInterface,
            library: ::godot_ffi::GDNativeExtensionClassLibraryPtr,
            init: *mut ::godot_ffi::GDNativeInitialization,
        ) -> ::godot_ffi::GDNativeBool {
            ::godot_core::init::__gdext_load_library::<#impl_ty>(
                interface,
                library,
                init
            )
        }

        fn __static_type_check() {
            // Ensures that the init function matches the signature advertised in FFI header
            let _unused: ::godot_ffi::GDNativeInitializationFunction = Some(#entry_point);
        }
    })
}
