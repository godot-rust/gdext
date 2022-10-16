/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{bail, ident};
use proc_macro2::{Delimiter, Group, TokenStream};
use quote::quote;
use std::mem;
use venial::Declaration;

pub fn transform(input: TokenStream) -> Result<TokenStream, venial::Error> {
    let decl = venial::parse_declaration(input)?;

    let func = match decl {
        Declaration::Function(f) => f,
        _ => return bail("#[gdextension] can only be applied to functions", &decl),
    };

    if !func.attributes.is_empty()
        || func.generic_params.is_some()
        || func.qualifiers.tk_default.is_some()
        || func.qualifiers.tk_const.is_some()
        || func.qualifiers.tk_async.is_some()
        || func.qualifiers.tk_unsafe.is_some()
        || func.qualifiers.tk_extern.is_some()
        || func.qualifiers.extern_abi.is_some()
        || func.return_ty.is_some()
        || func.where_clause.is_some()
    {
        return bail(
            &format!(
                "#[gdextension] function signature must be of these two:\n\
                  \tfn {f}(handle: &mut InitHandle) {{ ... }}\n\
                  \tfn {f}(handle: &mut InitHandle);",
                f = func.name
            ),
            &func,
        );
    }

    let mut func = func;
    if func.body.is_none() {
        let delim = Delimiter::Brace;
        let body = quote! {
            godot_core::init::__gdext_default_init(handle);
        };

        func.body = Some(Group::new(delim, body));
        func.tk_semicolon = None;
    }

    let internal_func_name = ident("__gdext_user_init");
    let extern_fn_name = mem::replace(&mut func.name, internal_func_name.clone());

    Ok(quote! {
        #func

        #[no_mangle]
        unsafe extern "C" fn #extern_fn_name(
            interface: *const ::godot_ffi::GDNativeInterface,
            library: ::godot_ffi::GDNativeExtensionClassLibraryPtr,
            init: *mut ::godot_ffi::GDNativeInitialization,
        ) -> ::godot_ffi::GDNativeBool {
            ::godot_core::init::__gdext_load_library(
                #internal_func_name,
                interface,
                library,
                init
            )
        }

        #[allow(dead_code)]
        const fn __gdext_static_type_check() {
            // Ensures that the init function matches the signature advertised in FFI header
            let _unused: ::godot_ffi::GDNativeInitializationFunction = Some(#extern_fn_name);
        }
    })
}
