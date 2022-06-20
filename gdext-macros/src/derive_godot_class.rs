use proc_macro2::TokenStream;
use quote::quote;
use venial::Error;

pub fn derive_native_class(input: TokenStream) -> Result<TokenStream, Error> {
    let class_decl = venial::parse_declaration(input)?;

    let class = class_decl.as_struct()?;
    let class_name = &class.name;
    let class_name_str = class.name.to_string();
    let fields = class.field_tokens();

    quote! {
        pub struct #class_name {
            #fields
        }
        impl GodotClass for ObjPayload {
            type Base = Node3D;
            type Declarer = marker::UserClass;
            type Mem = mem::ManualMemory;

            fn class_name() -> String {
                #class_name_str.to_string()
            }
        }
        impl GodotExtensionClass for #class_name {
            fn virtual_call(_name: &str) -> sys::GDNativeExtensionClassCallVirtual {
                todo!()
            }
            fn register_methods() {}
        }
        impl DefaultConstructible for ObjPayload {
            fn construct(_base: sys::GDNativeObjectPtr) -> Self {
                #class_name { }
            }
        }
    }
}
