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
use venial::{Declaration, StructFields};

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
        strip_attributes(&mut decl);
        strip_fields(&mut decl);
        Ok(decl.to_token_stream())
    }
}

fn strip_attributes(decl: &mut Declaration) {
    const GODOT_CLASS_ATTRS: [&str; 6] =
        ["class", "property", "export", "base", "signal", "godot_cfg"];

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
    const GODOT_CLASS_FIELDS: [&str; 1] = ["base"];
    const GODOT_CLASS_ATTRS: [&str; 1] = ["export"];

    let Declaration::Struct(class) = decl else {
        return;
    };

    let StructFields::Named(fields) = &mut class.fields else {
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
