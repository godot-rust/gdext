/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;
use venial::Declaration;

use crate::util::{bail, ident, validate_impl, KvParser};
use crate::ParseResult;

pub fn transform(decl: Declaration) -> ParseResult<TokenStream> {
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

    let drained_attributes = std::mem::take(&mut impl_decl.attributes);
    let mut parser = KvParser::parse_required(&drained_attributes, "gdextension", &impl_decl)?;
    let entry_point = parser.handle_ident("entry_point")?;
    parser.finish()?;

    let entry_point = entry_point.unwrap_or_else(|| ident("gdext_rust_init"));
    let impl_ty = &impl_decl.self_ty;

    Ok(quote! {
        #impl_decl

        #[no_mangle]
        unsafe extern "C" fn #entry_point(
            interface: *const ::godot::sys::GDExtensionInterface,
            library: ::godot::sys::GDExtensionClassLibraryPtr,
            init: *mut ::godot::sys::GDExtensionInitialization,
        ) -> ::godot::sys::GDExtensionBool {
            ::godot::init::__gdext_load_library::<#impl_ty>(
                interface,
                library,
                init
            )
        }

        fn __static_type_check() {
            // Ensures that the init function matches the signature advertised in FFI header
            let _unused: ::godot::sys::GDExtensionInitializationFunction = Some(#entry_point);
        }
    })
}
