use crate::util::bail;
use proc_macro2::TokenStream;
use quote::quote;
use venial::{Error, ImplMember};

pub fn transform(input: TokenStream) -> Result<TokenStream, Error> {
    let input_decl = venial::parse_declaration(input)?;
    let decl = match input_decl.as_impl() {
        Some(decl) => decl,
        None => bail(
            "#[godot_api] can only be applied on impl blocks",
            input_decl,
        )?,
    };

    if decl.trait_name.is_some() {
        bail(
            "#[godot_api] can only be applied on inherent impls, not on trait impls",
            decl,
        )?;
    }

    if decl.impl_generic_params.is_some() || decl.self_generic_args.is_some() {
        bail(
            "#[godot_api] currently does not support generic arguments",
            decl,
        )?;
    }

    let class_name = &decl.self_name;

    let mut methods = vec![];
    for item in decl.body.members.iter() {
        if let ImplMember::Method(method) = item {
            let mut godot_attr_iter = method
                .attributes
                .iter()
                .filter(|attr| attr.get_single_path_segment().unwrap() == "godot");

            let _godot_attr = if let Some(found) = godot_attr_iter.next() {
                let val = found.value.clone();
                if godot_attr_iter.next().is_some() {
                    bail("at most one #[godot] attribute per method allowed", method)?;
                }
                val
            } else {
                continue;
            };

            let mut method = method.clone();
            method.body = None;
            methods.push(method);
        }
    }

    let result = quote! {
        #decl

        impl gdext_class::traits::GodotExtensionClass for #class_name {
            fn virtual_call(name: &str) -> sys::GDNativeExtensionClassCallVirtual {
                println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);

                None // TODO
            }
            fn register_methods() {
                #(
                    gdext_class::gdext_wrap_method!(#class_name, #methods);
                )*
            }
        }
    };

    Ok(result)
}
