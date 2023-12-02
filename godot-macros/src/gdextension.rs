/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;
use venial::Declaration;

use crate::util::{bail, ident, validate_impl, KvParser};
use crate::ParseResult;

pub fn attribute_gdextension(decl: Declaration) -> ParseResult<TokenStream> {
    let mut impl_decl = match decl {
        Declaration::Impl(item) => item,
        _ => return bail!(&decl, "#[gdextension] can only be applied to trait impls"),
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
    parser.finish()?;

    let entry_point = entry_point.unwrap_or_else(|| ident("gdext_rust_init"));
    let impl_ty = &impl_decl.self_ty;

    Ok(quote! {
        #impl_decl

        // This cfg cannot be checked from the outer proc-macro since its 'target' is the build
        // host. See: https://github.com/rust-lang/rust/issues/42587
        #[cfg(target_os = "emscripten")]
        fn emscripten_preregistration() {
            // Module is documented here[1] by emscripten so perhaps we can consider it a part
            // of its public API? In any case for now we mutate global state directly in order
            // to get things working.
            // [1] https://emscripten.org/docs/api_reference/module.html
            //
            // Warning: It may be possible that in the process of executing the code leading up
            // to `emscripten_run_script` that we might trigger usage of one of the symbols we
            // wish to monkey patch? It seems fairly unlikely, especially as long as no i64 are
            // involved, but I don't know what guarantees we have here.
            //
            // We should keep an eye out for these sorts of failures!
            let script = std::ffi::CString::new(concat!(
                "var pkgName = '", env!("CARGO_PKG_NAME"), "';", r#"
                var libName = pkgName.replaceAll('-', '_') + '.wasm';
                var dso = LDSO.loadedLibsByName[libName];
                // This property was renamed as of emscripten 3.1.34
                var dso_exports = "module" in dso ? dso["module"] : dso["exports"];
                var registrants = [];
                for (sym in dso_exports) {
                    if (sym.startsWith("dynCall_")) {
                        if (!(sym in Module)) {
                            console.log(`Patching Module with ${sym}`);
                            Module[sym] = dso_exports[sym];
                        }
                    } else if (sym.startsWith("rust_gdext_registrant_")) {
                        registrants.push(sym);
                    }
                }
                for (sym of registrants) {
                    console.log(`Running registrant ${sym}`);
                    dso_exports[sym]();
                }
                console.log("Added",  registrants.length, "plugins to registry!");
            "#)).expect("Unable to create CString from script");

            extern "C" { fn emscripten_run_script(script: *const std::ffi::c_char); }
            unsafe { emscripten_run_script(script.as_ptr()); }
        }

        #[no_mangle]
        unsafe extern "C" fn #entry_point(
            interface_or_get_proc_address: ::godot::sys::InitCompat,
            library: ::godot::sys::GDExtensionClassLibraryPtr,
            init: *mut ::godot::sys::GDExtensionInitialization,
        ) -> ::godot::sys::GDExtensionBool {
            // Required due to the lack of a constructor facility such as .init_array in rust wasm
            #[cfg(target_os = "emscripten")]
            emscripten_preregistration();

            ::godot::init::__gdext_load_library::<#impl_ty>(
                interface_or_get_proc_address,
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
