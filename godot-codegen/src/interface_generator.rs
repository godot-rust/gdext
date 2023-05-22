/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::central_generator::write_file;
use crate::util::ident;
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

struct GodotFuncPtr {
    name: Ident,
    func_ptr_ty: Ident,
    doc: String,
}

pub(crate) fn generate_sys_interface_file(
    h_path: &Path,
    sys_gen_path: &Path,
    is_godot_4_0: bool,
    out_files: &mut Vec<PathBuf>,
) {
    let code = if is_godot_4_0 {
        // Compat for 4.0.x
        // Most polyfills are in godot_exe.rs, fn polyfill_legacy_header()
        quote! {
            #[path = "../compat/compat_4_0.rs"]
            mod compat_4_0;

            pub use compat_4_0::InitCompat;
        }
    } else {
        generate_proc_address_funcs(h_path)
    };

    write_file(sys_gen_path, "interface.rs", code.to_string(), out_files);
}

fn generate_proc_address_funcs(h_path: &Path) -> TokenStream {
    let header_code = fs::read_to_string(h_path)
        .expect("failed to read gdextension_interface.h for header parsing");
    let func_ptrs = parse_function_pointers(&header_code);

    let mut fptr_decls = vec![];
    let mut fptr_inits = vec![];
    for fptr in func_ptrs {
        let GodotFuncPtr {
            name,
            func_ptr_ty,
            doc,
        } = fptr;

        let name_str = Literal::byte_string(format!("{}\0", name).as_bytes());

        let decl = quote! {
            #[doc = #doc]
            pub #name: crate::#func_ptr_ty,
        };

        // SAFETY: transmute relies on Option<F1> and Option<F2> having the same layout.
        // It might be better to transmute the raw function pointers, but then we have no type names.
        let init = quote! {
            #name: std::mem::transmute::<
                crate::GDExtensionInterfaceFunctionPtr,
                crate::#func_ptr_ty
            >(get_proc_address(crate::c_str(#name_str))),
        };

        fptr_decls.push(decl);
        fptr_inits.push(init);
    }

    // Do not derive Copy -- even though the struct is bitwise-copyable, this is rarely needed and may point to an error.
    let code = quote! {
        #[path = "../compat/compat_4_1.rs"]
        mod compat_4_1;

        pub use compat_4_1::InitCompat;

        pub struct GDExtensionInterface {
            #( #fptr_decls )*
        }

        impl GDExtensionInterface {
            pub(crate) unsafe fn load(
                get_proc_address: crate::GDExtensionInterfaceGetProcAddress,
            ) -> Self {
                let get_proc_address = get_proc_address.expect("invalid get_proc_address function pointer");

                Self {
                    #( #fptr_inits )*
                }
            }
        }
    };
    code
}

fn parse_function_pointers(header_code: &str) -> Vec<GodotFuncPtr> {
    // See https://docs.rs/regex/latest/regex for docs.
    let regex = Regex::new(
        r#"(?xms)
        # x: ignore whitespace and allow line comments (starting with `#`)
        # m: multi-line mode, ^ and $ match start and end of line
        # s: . matches newlines; would otherwise require (:?\n|\r\n|\r)
        ^
        # Start of comment           /**
        /\*\*
        # followed by any characters
        [^*].*?
        # Identifier                 @name variant_can_convert
        @name\s(?P<name>[a-z0-9_]+)
        (?P<doc>
            .+?
        )
        #(?:@param\s([a-z0-9_]+))*?
        #(?:\n|.)+?
        # End of comment             */
        \*/
        .+?
        # Return type:               typedef GDExtensionBool
        # or pointers with space:    typedef void *
        #typedef\s[A-Za-z0-9_]+?\s\*?
        typedef\s[^(]+?
        # Function pointer:          (*GDExtensionInterfaceVariantCanConvert)
        \(\*(?P<type>[A-Za-z0-9_]+?)\)
        # Parameters:                (GDExtensionVariantType p_from, GDExtensionVariantType p_to);
        .+?;
        # $ omitted, because there can be comments after `;`
    "#,
    )
    .unwrap();

    let mut func_ptrs = vec![];
    for cap in regex.captures_iter(header_code) {
        let name = cap.name("name");
        let funcptr_ty = cap.name("type");
        let doc = cap.name("doc");

        let (Some(name), Some(funcptr_ty), Some(doc)) = (name, funcptr_ty, doc) else {
			// Skip unparseable ones, instead of breaking build (could just be a /** */ comment around something else)
			continue;
		};

        func_ptrs.push(GodotFuncPtr {
            name: ident(name.as_str()),
            func_ptr_ty: ident(funcptr_ty.as_str()),
            doc: doc.as_str().replace("\n *", "\n").trim().to_string(),
        });
    }

    func_ptrs
}

// fn doxygen_to_rustdoc(c_doc: &str) -> String {
//     // Remove leading stars
//     let mut doc = c_doc .replace("\n * ", "\n");
//
//     // FIXME only compile once
//     let param_regex = Regex::new(r#"@p"#)
// }

#[test]
fn test_parse_function_pointers() {
    let header_code = r#"
/* INTERFACE: ClassDB Extension */

/**
 * @name classdb_register_extension_class
 *
 * Registers an extension class in the ClassDB.
 *
 * Provided struct can be safely freed once the function returns.
 *
 * @param p_library A pointer the library received by the GDExtension's entry point function.
 * @param p_class_name A pointer to a StringName with the class name.
 * @param p_parent_class_name A pointer to a StringName with the parent class name.
 * @param p_extension_funcs A pointer to a GDExtensionClassCreationInfo struct.
 */
typedef void (*GDExtensionInterfaceClassdbRegisterExtensionClass)(GDExtensionClassLibraryPtr p_library, GDExtensionConstStringNamePtr p_class_name, GDExtensionConstStringNamePtr p_parent_class_name, const GDExtensionClassCreationInfo *p_extension_funcs);
		"#;

    let func_ptrs = parse_function_pointers(header_code);
    assert_eq!(func_ptrs.len(), 1);

    let func_ptr = &func_ptrs[0];
    assert_eq!(
        func_ptr.name.to_string(),
        "classdb_register_extension_class"
    );

    assert_eq!(
        func_ptr.func_ptr_ty.to_string(),
        "GDExtensionInterfaceClassdbRegisterExtensionClass"
    );

    assert_eq!(
        func_ptr.doc,
        r#"
 Registers an extension class in the ClassDB.

 Provided struct can be safely freed once the function returns.

 @param p_library A pointer the library received by the GDExtension's entry point function.
 @param p_class_name A pointer to a StringName with the class name.
 @param p_parent_class_name A pointer to a StringName with the parent class name.
 @param p_extension_funcs A pointer to a GDExtensionClassCreationInfo struct.
		 "#
        .trim()
    );
}
