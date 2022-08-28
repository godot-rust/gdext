use crate::util::bail;
use proc_macro2::{Punct, Spacing, TokenStream};
use quote::quote;
use venial::{Declaration, Error, Function, Impl, ImplMember};

pub fn transform(input: TokenStream) -> Result<TokenStream, Error> {
    let input_decl = venial::parse_declaration(input)?;
    let decl = match input_decl {
        Declaration::Impl(decl) => decl,
        _ => bail(
            "#[godot_api] can only be applied on impl blocks",
            input_decl,
        )?,
    };

    if decl.impl_generic_params.is_some() {
        bail(
            "#[godot_api] currently does not support generic parameters",
            &decl,
        )?;
    }

    if decl.self_ty.as_path().is_none() {
        return bail("invalid Self type for #[godot_api] impl", decl);
    };

    if decl.trait_ty.is_some() {
        transform_trait_impl(decl)
    } else {
        transform_inherent_impl(decl)
    }
}

/// Codegen for `#[godot_api] impl MyType`
fn transform_inherent_impl(mut decl: Impl) -> Result<TokenStream, Error> {
    let methods = process_godot_fns(&mut decl)?;
    let self_class = &decl.self_ty;
    let result = quote! {
        #decl

        impl gdext_class::traits::UserMethodBinds for #self_class {

            fn register_methods() {
                #(
                    gdext_class::gdext_register_method!(#self_class, #methods);
                    //#methods
                )*
            }
        }
    };

    Ok(result)
}

/// Codegen for `#[godot_api] impl GodotMethods for MyType`
fn transform_trait_impl(decl: Impl) -> Result<TokenStream, Error> {
    match decl.trait_ty.as_ref().expect("Impl::trait_ty").as_path() {
        Some(path) => path
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "GodotMethods"),
        _ => bail(
            "#[godot_api] for trait impls requires trait to be `GodotMethods`",
            &decl,
        )?,
    };

    //let mut godot_default = TokenStream::new();

    let self_class = &decl.self_ty;
    for item in decl.body_items.iter() {
        let method = if let ImplMember::Method(f) = item {
            f
        } else {
            continue;
        };

        match method.name.to_string().as_str() {
            "init" => {}
            "to_string" => {}
            m => return bail(format!("Unsupported GodotMethods method: {}", m), method),
        }
    }

    let result = quote! {
        #decl
        //#godot_default

        impl gdext_class::traits::UserVirtuals for #self_class {
            fn virtual_call(name: &str) -> gdext_sys::GDNativeExtensionClassCallVirtual {
                println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);

                None // TODO
            }
        }
    };

    Ok(result)
}

fn process_godot_fns(decl: &mut Impl) -> Result<Vec<Function>, Error> {
    let mut method_signatures = vec![];
    for item in decl.body_items.iter_mut() {
        let method = if let ImplMember::Method(method) = item {
            method
        } else {
            continue;
        };

        let mut found = None;
        for (index, attr) in method.attributes.iter().enumerate() {
            if attr
                .get_single_path_segment()
                .expect("get_single_path_segment")
                == "godot"
            {
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
            sig.tk_semicolon = Some(Punct::new(';', Spacing::Alone));
            method_signatures.push(sig);
        }
    }

    Ok(method_signatures)
}
