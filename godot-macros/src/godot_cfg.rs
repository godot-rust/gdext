mod all;
mod any;
mod error;
mod list;
mod not;
mod option;
mod predicate;

use crate::ParseResult;
use all::*;
use any::*;
use error::*;
use list::*;
use not::*;
use option::*;
use predicate::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use venial::{Declaration, ImplMember, StructFields};

trait GodotConditionalCompilation {
    fn should_compile(&self) -> bool;
}

pub fn should_compile(ts: proc_macro2::TokenStream) -> Result<bool, venial::Error> {
    let predicate = GodotConfigurationPredicate::try_from(ts)?;
    Ok(predicate.should_compile())
}

pub fn transform(meta: TokenStream, input: TokenStream) -> ParseResult<TokenStream> {
    if should_compile(meta)? {
        Ok(input)
    } else {
        let mut decl = venial::parse_declaration(input)?;
        match &decl {
            Declaration::Struct(_) => {
                strip_attributes(&mut decl);
                strip_fields(&mut decl);
            }
            Declaration::Impl(impl_decl) => {
                // Dump GodotExt all together as this can not be
                if let Some(path) = impl_decl.trait_ty.as_ref().map(|ty| ty.to_token_stream().to_string()) {
                    if path == "GodotExt" {
                        return Ok(TokenStream::new())
                    }
                }
                strip_attributes(&mut decl);
                strip_methods(&mut decl);
            }
            _ => {}
        }
        Ok(decl.to_token_stream())
    }
}

fn strip_attributes(decl: &mut Declaration) {
    const GODOT_CLASS_ATTRS: [&str; 7] =
        ["class", "property", "export", "base", "signal", "godot_api", "godot_cfg"];

    let mut remove_attrs = vec![];
    for (index, attr) in decl.attributes().iter().enumerate().rev() {
        let path = attr.path[0].to_string();
        if GODOT_CLASS_ATTRS.contains(&path.as_str()) {
            remove_attrs.push(index)
        }
    }
    remove_attrs.into_iter().for_each(|index| {
        decl.attributes_mut().remove(index);
    });
}

fn strip_fields(decl: &mut Declaration) {
    // Things that should be removed completely, you will need to stub these
    const GODOT_CLASS_FIELDS: [&str; 1] = ["base"];
    // Things we can just remove the attribute annotation from
    const GODOT_CLASS_ATTRS: [&str; 1] = ["export"];

    let Declaration::Struct(struct_decl) = decl else {
        return;
    };

    let StructFields::Named(fields) = &mut struct_decl.fields else {
        return;
    };

    let mut remove_fields = vec![];
    for (f_index, (field, _)) in fields.fields.inner.iter_mut().enumerate().rev() {
        let mut remove_attrs = vec![];
        for (a_index, attr) in field.attributes.iter().enumerate() {
            if attr
                .get_single_path_segment()
                .map(|path| GODOT_CLASS_FIELDS.contains(&path.to_string().as_str()))
                .unwrap_or(false)
            {
                remove_fields.push(f_index);
            } else if attr
                .get_single_path_segment()
                .map(|path| GODOT_CLASS_ATTRS.contains(&path.to_string().as_str()))
                .unwrap_or(false)
            {
                remove_attrs.push(a_index);
            }
        }
        remove_attrs.into_iter().for_each(|index| {
            field.attributes.remove(index);
        })
    }
    remove_fields.into_iter().for_each(|index| {
        fields.fields.inner.remove(index);
    })
}

fn strip_methods(decl: &mut Declaration) {
    // Things that should be removed completely, you will need to stub these
    const GODOT_CLASS_FIELDS: [&str; 1] = ["signal"];
    // Things we can just remove the attribute annotation from
    const GODOT_CLASS_ATTRS: [&str; 1] = ["func"];

    let Declaration::Impl(impl_decl) = decl else {
        return;
    };

    let mut remove_methods = vec![];
    for (m_index, member) in impl_decl.body_items.iter_mut().enumerate() {
        let ImplMember::Method(method) = member else {
            continue;
        };

        let mut remove_attrs = vec![];
        for (a_index, attr) in method.attributes.iter().enumerate() {
            if attr
                .get_single_path_segment()
                .map(|path| GODOT_CLASS_FIELDS.contains(&path.to_string().as_str()))
                .unwrap_or(false)
            {
                remove_methods.push(m_index);
            } else if attr
                .get_single_path_segment()
                .map(|path| GODOT_CLASS_ATTRS.contains(&path.to_string().as_str()))
                .unwrap_or(false)
            {
                remove_attrs.push(a_index);
            }
        }
        remove_attrs.into_iter().for_each(|index| {
            method.attributes.remove(index);
        })
    }
    remove_methods.into_iter().for_each(|index| {
        impl_decl.body_items.remove(index);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    #[test]
    fn test_should_compile() {
        let ts = TokenStream::from_str("any(all(test, not(doctest)), doctest)").unwrap();
        assert!(should_compile(ts).unwrap());
    }
}
