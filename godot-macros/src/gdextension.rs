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

        // This cfg cannot be checked from the outer proc-macro since its 'target' is the build
        // host. See: https://github.com/rust-lang/rust/issues/42587
        #[cfg(target_os = "emscripten")]
        fn emscripten_preregistration() {
            let pkg_name = env!("CARGO_PKG_NAME");
            let wasm_binary = <#impl_ty as ::godot::init::ExtensionLibrary>::override_wasm_binary()
                .map_or_else(
                    || std::string::String::from("null"),
                    |bin| format!("'{}'", bin.replace("\\", "\\\\").replace("'", "\\'"))
                );

            // Module is documented here[1] by emscripten, so perhaps we can consider it a part
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
            let script = format!(
                r#"var pkgName = '{pkg_name}';
                var wasmBinary = {wasm_binary};
                if (wasmBinary === null) {{
                    var snakePkgName = pkgName.replaceAll('-', '_');
                    var normalLibName = snakePkgName + '.wasm';
                    var threadedLibName = snakePkgName + '.threads.wasm';
                    if (normalLibName in LDSO.loadedLibsByName) {{
                        var libName = normalLibName;
                    }} else if (threadedLibName in LDSO.loadedLibsByName) {{
                        var libName = threadedLibName;
                    }} else {{
                        // Always print to console, even if the error is suppressed.
                        console.error(`godot-rust could not find the Wasm module '${{normalLibName}}' nor '${{threadedLibName}}', one of which is needed by default to load the '${{pkgName}}' crate. This indicates its '.wasm' binary file was renamed to an unexpected name.\n\nPlease ensure its Wasm binary file has one of those names in the game's web export files. This may require updating Wasm paths in the crate's corresponding '.gdextension' file, or just renaming the Wasm file to one of the expected names otherwise.\n\nIf that GDExtension uses a different Wasm filename, please ensure it informs this new name to godot-rust by returning 'Some("newname.wasm")' from 'ExtensionLibrary::override_wasm_binary'.`);
                        throw new Error(`Wasm module '${{normalLibName}}' not found. Check the console for more information.`);
                    }}
                }} else if (!wasmBinary.endsWith(".wasm")) {{
                    console.error(`godot-rust received an invalid Wasm binary name ('${{wasmBinary}}') from crate '${{pkgName}}', as the '.wasm' extension was missing.\n\nPlease ensure the 'ExtensionLibrary::override_wasm_binary' function for that GDExtension always returns a filename with the '.wasm' extension and try again.`);
                    throw new Error(`Invalid Wasm module '${{wasmBinary}}' (missing '.wasm' extension). Check the console for more information.`);
                }} else if (wasmBinary in LDSO.loadedLibsByName) {{
                    var libName = wasmBinary;
                }} else {{
                    console.error(`godot-rust could not find the Wasm module '${{wasmBinary}}', needed to load the '${{pkgName}}' crate. This indicates its '.wasm' binary file was renamed to an unexpected name.\n\nPlease ensure its Wasm binary file is named '${{wasmBinary}}' in the game's web export files. This may require updating Wasm paths in the crate's corresponding '.gdextension' file, or just renaming the Wasm file to the expected name otherwise.`);
                    throw new Error(`Wasm module '${{wasmBinary}}' not found. Check the console for more information.`);
                }}
                var dso = LDSO.loadedLibsByName[libName];
                // This property was renamed as of emscripten 3.1.34
                var dso_exports = "module" in dso ? dso["module"] : dso["exports"];
                var registrants = [];
                for (sym in dso_exports) {{
                    if (sym.startsWith("dynCall_")) {{
                        if (!(sym in Module)) {{
                            console.log(`Patching Module with ${{sym}}`);
                            Module[sym] = dso_exports[sym];
                        }}
                    }} else if (sym.startsWith("__godot_rust_registrant_")) {{
                        registrants.push(sym);
                    }}
                }}
                for (sym of registrants) {{
                    console.log(`Running registrant ${{sym}}`);
                    dso_exports[sym]();
                }}
                console.log("Added",  registrants.length, "plugins to registry!");
            "#);

            let script = std::ffi::CString::new(script).expect("Unable to create CString from script");

            extern "C" { fn emscripten_run_script(script: *const std::ffi::c_char); }
            unsafe { emscripten_run_script(script.as_ptr()); }
        }

        #[no_mangle]
        unsafe extern "C" fn #entry_point(
            get_proc_address: ::godot::sys::GDExtensionInterfaceGetProcAddress,
            library: ::godot::sys::GDExtensionClassLibraryPtr,
            init: *mut ::godot::sys::GDExtensionInitialization,
        ) -> ::godot::sys::GDExtensionBool {
            // Required due to the lack of a constructor facility such as .init_array in rust wasm
            #[cfg(target_os = "emscripten")]
            emscripten_preregistration();

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
