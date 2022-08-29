use crate::util::bail;
use crate::{util, ParseResult};
use proc_macro2::{Ident, Punct, Spacing, TokenStream};
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
fn transform_trait_impl(original_impl: Impl) -> Result<TokenStream, Error> {
    let class_name = validate_trait_impl(&original_impl)?;
    let class_name_str = class_name.to_string();

    let mut godot_init_impl = TokenStream::new();
    let mut create_fn = quote! { None };
    let mut to_string_fn = quote! { None };
    let mut virtual_methods = vec![];
    let mut virtual_method_names = vec![];

    let prv = quote! { gdext_class::private };

    for item in original_impl.body_items.iter() {
        let method = if let ImplMember::Method(f) = item {
            f
        } else {
            continue;
        };

        let method_name = method.name.to_string();
        match method_name.as_str() {
            "init" => {
                godot_init_impl = quote! {
                    impl gdext_class::traits::GodotDefault for #class_name {
                        fn godot_default(base: gdext_class::Base<Self::Base>) -> Self {
                            <Self as gdext_class::traits::GodotMethods>::init(base)
                        }
                    }
                };
                create_fn = quote! { Some(#prv::c_api::create::<#class_name>) };
            }

            "to_string" => {
                to_string_fn = quote! { Some(#prv::c_api::to_string::<#class_name>) };
            }

            // Other virtual methods, like ready, process etc.
            known_name if VIRTUAL_METHOD_NAMES.contains(&known_name) => {
                let method = util::reduce_to_signature(method);

                virtual_method_names.push(method_name);
                virtual_methods.push(method);
            }

            // Unknown methods which are declared inside trait impl are not supported (possibly compiler catches those first anyway)
            other_name => {
                return bail(
                    format!("Unsupported GodotMethods method: {}", other_name),
                    method,
                )
            }
        }
    }

    let result = quote! {
        #original_impl
        #godot_init_impl

        impl gdext_class::traits::UserVirtuals for #class_name {
            fn virtual_call(name: &str) -> gdext_sys::GDNativeExtensionClassCallVirtual {
                println!("virtual_call: {}.{}", std::any::type_name::<Self>(), name);

                match name {
                    #(
                        #virtual_method_names => gdext_class::gdext_virtual_method_callback!(#class_name, #virtual_methods),
                    )*
                    _ => None,
                }
            }
        }

        gdext_sys::plugin_add!(GDEXT_CLASS_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_str,
            component: #prv::PluginComponent::UserVirtuals {
                user_create_fn: #create_fn,
                to_string_fn: #to_string_fn,
                get_virtual_fn: #prv::c_api::get_virtual::<#class_name>,
            },
        });
    };

    Ok(result)
}

/// Make sure that in `impl Trait for Self`, both `Trait` and `Self` are good
fn validate_trait_impl(original_impl: &Impl) -> ParseResult<Ident> {
    // impl Trait for Self -- validate Trait
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    if !extract_typename(&trait_name).map_or(false, |seg| seg.ident == "GodotMethods") {
        return bail(
            "#[godot_api] for trait impls requires trait to be `GodotMethods`",
            &original_impl,
        );
    }

    // impl Trait for Self -- validate Self
    if let Some(segment) = extract_typename(&original_impl.self_ty) {
        if segment.generic_args.is_none() {
            Ok(segment.ident)
        } else {
            bail(
                "#[godot_api] for trait impls does currently not support generic arguments",
                &original_impl,
            )
        }
    } else {
        bail(
            "#[godot_api] for trait impls requires Self type to be a simple path",
            &original_impl,
        )
    }
}

/// Gets the right-most type name in the path
fn extract_typename(ty: &venial::TyExpr) -> Option<venial::PathSegment> {
    match ty.as_path() {
        Some(mut path) => path.segments.pop(),
        _ => None,
    }
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

// Note: keep in sync with trait GodotMethods
const VIRTUAL_METHOD_NAMES: [&'static str; 3] = ["ready", "process", "physics_process"];
