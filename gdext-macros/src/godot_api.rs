use crate::util::bail;
use proc_macro2::TokenStream;
use quote::quote;
use venial::{ Declaration, Error, Function, Impl, ImplMember};

pub fn transform(input: TokenStream) -> Result<TokenStream, Error> {
    let input_decl = venial::parse_declaration(input)?;
    let mut decl = match input_decl {
        Declaration::Impl(decl) => decl,
        _ => bail(
            "#[godot_api] can only be applied on impl blocks",
            input_decl,
        )?,
    };

    if decl.trait_name.is_some() {
        bail(
            "#[godot_api] can only be applied on inherent impls, not on trait impls",
            &decl,
        )?;
    }

    if decl.impl_generic_params.is_some() || decl.self_generic_args.is_some() {
        bail(
            "#[godot_api] currently does not support generic arguments",
            &decl,
        )?;
    }

    let methods = process_godot_fns(&mut decl)?;

    let class_name = &decl.self_name;
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
                    //#methods
                )*
            }
        }
    };

    Ok(result)
}

fn process_godot_fns(decl: &mut Impl) -> Result<Vec<Function>, Error> {
    let mut method_signatures = vec![];
    for item in decl.body.members.iter_mut() {
        let method = if let ImplMember::Method(method) = item {
            method
        } else {
            continue;
        };

        let mut found = None;
        for (index, attr) in method.attributes.iter().enumerate() {
            if attr.get_single_path_segment().unwrap() == "godot" {
                if found.is_some() {
                    bail("at most one #[godot] attribute per method allowed", &method)?;
                } else {
                    found = Some((index, attr.value.clone()));
                }
            }
        }

        if let Some((index, _attr_val)) = found {
            // Remaining code no longer has attribute -- rest stays
            method.attributes.remove(index);

            // Signatures are the same thing without body
            let mut sig = method.clone();
            sig.body = None;
            method_signatures.push(sig);
        }
    }

    Ok(method_signatures)
}
