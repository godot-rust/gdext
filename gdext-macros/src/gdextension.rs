use proc_macro::TokenTree;
use crate::util::bail;
use proc_macro2::TokenStream;
use quote::quote;
use venial::{Declaration, ImplMember};

pub fn transform(input: TokenStream) -> Result<TokenStream, venial::Error> {
    let decl = venial::parse_declaration(input)?;

    let impl_decl = match decl {
        Declaration::Impl(item) => item,
        _ => return bail("#[gdextension] can only be applied to trait impls", &decl),
    };

    for item in impl_decl.body_items {
        match item {
            ImplMember::Constant(c) => {
                if c.name == "ENTRY_POINT" {
                    if let Some(entry_point) = c.initializer {
                        let iter = entry_point.tokens.iter();
                        match iter.next() {
                            TokenTree::Literal(lit) => {
                                //lit.
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }


    Ok(quote! {
        #[no_mangle]
        unsafe extern "C" fn #entry_point_fn(
            interface: *const ::gdext_sys::GDNativeInterface,
            library: ::gdext_sys::GDNativeExtensionClassLibraryPtr,
            init: *mut ::gdext_sys::GDNativeInitialization,
        ) -> ::gdext_sys::GDNativeBool {
            ::gdext_class::init::__gdext_load_library::<#impl_ty>(
                interface,
                library,
                init
            )
        }

        fn __static_type_check() {
            // Ensures that the init function matches the signature advertised in FFI header
            let _unused: ::gdext_sys::GDNativeInitializationFunction = Some(load_library);
        }
    })
}
