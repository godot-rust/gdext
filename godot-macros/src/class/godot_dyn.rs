/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::bail;
use crate::{util, ParseResult};
use proc_macro2::TokenStream;
use quote::quote;

pub fn attribute_godot_dyn(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let venial::Item::Impl(decl) = input_decl else {
        return bail!(
            input_decl,
            "#[godot_dyn] can only be applied on impl blocks",
        );
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "#[godot_dyn] currently does not support generic parameters",
        )?;
    }

    let Some(trait_path) = decl.trait_ty.as_ref() else {
        return bail!(
            &decl,
            "#[godot_dyn] requires a trait; it cannot be applied to inherent impl blocks",
        );
    };

    let class_path = &decl.self_ty;
    let class_name_obj = util::class_name_obj(class_path); //&util::extract_typename(class_path));
    let prv = quote! { ::godot::private };

    //let dynify_fn = format_ident!("__dynify_{}", class_name);

    let new_code = quote! {
        #decl

        impl ::godot::obj::AsDyn<dyn #trait_path> for #class_path {
            fn dyn_upcast(&self) -> &(dyn #trait_path + 'static) {
                self
            }

            fn dyn_upcast_mut(&mut self) -> &mut (dyn #trait_path + 'static) {
                self
            }
        }

        ::godot::sys::plugin_add!(__GODOT_PLUGIN_REGISTRY in #prv; #prv::ClassPlugin {
            class_name: #class_name_obj,
            item: #prv::PluginItem::DynTraitImpl {
                dyn_trait_typeid: std::any::TypeId::of::<dyn #trait_path>(),
                erased_dynify_fn: {
                    fn dynify_fn(obj: ::godot::obj::Gd<::godot::classes::Object>) -> #prv::ErasedDynGd {
                        let obj = unsafe { obj.try_cast::<#class_path>().unwrap_unchecked() };
                        let obj = obj.into_dyn::<dyn #trait_path>();
                        let obj = obj.upcast::<::godot::classes::Object>();

                        #prv::ErasedDynGd {
                            boxed: Box::new(obj),
                        }
                    }

                    dynify_fn
                }
            },
            init_level: <#class_path as ::godot::obj::GodotClass>::INIT_LEVEL,
        });

    };

    Ok(new_code)
}
