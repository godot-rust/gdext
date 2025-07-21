/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::{bail, error, ident, is_punct, path_is_single, ListParser};
use crate::ParseResult;
use proc_macro2::{Delimiter, Ident, Literal, Spacing, Span, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::HashMap;

pub(crate) type KvMap = HashMap<Ident, Option<KvValue>>;

/// Struct to parse attributes like `#[attr(key, key2="value", key3=123)]` in a very user-friendly way.
pub(crate) struct KvParser {
    map: KvMap,
    span: Span,
}

#[allow(dead_code)] // some functions will be used later
impl KvParser {
    /// Create a new parser which requires a `#[expected]` attribute.
    ///
    /// `context` is used for the span in error messages.
    pub fn parse_required(
        attributes: &[venial::Attribute],
        expected: &str,
        context: impl ToTokens,
    ) -> ParseResult<Self> {
        match Self::parse(attributes, expected) {
            Ok(Some(result)) => Ok(result),
            Ok(None) => bail!(context, "expected attribute #[{expected}], but not present",),
            Err(e) => Err(e),
        }
    }

    /// Create a new parser which checks for presence of an `#[expected]` attribute.
    ///
    /// Returns `Ok(None)` if the attribute is not present.
    pub fn parse(attributes: &[venial::Attribute], expected: &str) -> ParseResult<Option<Self>> {
        let mut found_attr: Option<Self> = None;

        for attr in attributes.iter() {
            let path = &attr.path;
            if path_is_single(path, expected) {
                if found_attr.is_some() {
                    return bail!(attr, "only a single #[{expected}] attribute allowed");
                }

                let attr_name = expected.to_string();
                found_attr = Some(Self {
                    span: attr.tk_brackets.span,
                    map: ParserState::parse(attr_name, &attr.value)?,
                });
            }
        }

        Ok(found_attr)
    }

    /// Like `parse()`, but removes the attribute from the list.
    ///
    /// Useful for `#[proc_macro_attributes]`, where handled attributes must not show up in resulting code.
    // Currently unused.
    pub fn parse_remove(
        attributes: &mut Vec<venial::Attribute>,
        expected: &str,
    ) -> ParseResult<Option<Self>> {
        let mut found_attr: Option<Self> = None;
        let mut found_index: Option<usize> = None;

        for (i, attr) in attributes.iter().enumerate() {
            let path = &attr.path;
            if path_is_single(path, expected) {
                if found_attr.is_some() {
                    return bail!(attr, "only a single #[{expected}] attribute allowed");
                }

                let attr_name = expected.to_string();
                found_index = Some(i);
                found_attr = Some(Self {
                    span: attr.tk_brackets.span,
                    map: ParserState::parse(attr_name, &attr.value)?,
                });
            }
        }

        if let Some(index) = found_index {
            attributes.remove(index);
        }

        Ok(found_attr)
    }

    pub fn span(&self) -> Span {
        self.span
    }

    /// - For missing keys, returns `None`.
    /// - For a key with no value, returns `Some(None)`.
    /// - For a key with a value, returns `Some(value)`.
    pub fn handle_any(&mut self, key: &str) -> Option<Option<KvValue>> {
        self.map.remove(&ident(key))
    }

    /// Handle any value, returning both the key and value.
    ///
    /// - For missing keys, returns `None`.
    /// - For a key with no value, returns `Some((key, None))`.
    /// - For a key with a value, returns `Some((key, Some(value)))`.
    pub fn handle_any_entry(&mut self, key: &str) -> Option<(Ident, Option<KvValue>)> {
        self.map.remove_entry(&ident(key))
    }

    /// Handles a key that can only occur without a value, e.g. `#[attr(toggle)]`. Returns whether
    /// the key is present.
    pub fn handle_alone(&mut self, key: &str) -> ParseResult<bool> {
        self.handle_alone_with_span(key).map(|id| id.is_some())
    }

    /// Handles a key that can only occur without a value, e.g. `#[attr(toggle)]`. Returns the key (as an ident with a span)
    /// if it is present.
    pub fn handle_alone_with_span(&mut self, key: &str) -> ParseResult<Option<Ident>> {
        match self.handle_any_entry(key) {
            None => Ok(None),
            Some((id, value)) => match value {
                None => Ok(Some(id)),
                Some(value) => bail!(&value.tokens[0], "key `{key}` should not have a value"),
            },
        }
    }

    /// Handles an optional key that can only occur with an identifier as the value.
    pub fn handle_ident(&mut self, key: &str) -> ParseResult<Option<Ident>> {
        match self.map.remove_entry(&ident(key)) {
            None => Ok(None),
            // The `key` that was removed from the map has the correct span.
            Some((key, value)) => match value {
                None => bail!(key, "expected `{key}` to be followed by `= identifier`"),
                Some(value) => Ok(Some(value.ident()?)),
            },
        }
    }

    /// Handles an array of the form `[elem1, elem2, ...]`.
    pub fn handle_array(&mut self, key: &str) -> ParseResult<Option<ListParser>> {
        ListParser::new_from_kv(self, key, Delimiter::Bracket)
    }

    /// Handles an list of the form `(elem1, elem2, ...)`.
    pub fn handle_list(&mut self, key: &str) -> ParseResult<Option<ListParser>> {
        ListParser::new_from_kv(self, key, Delimiter::Parenthesis)
    }

    /// Handles an optional key that can occur with arbitrary tokens as the value.
    ///
    /// Returns both the key (with the correct span pointing to the attribute) and the value.    
    /// [KvParser.span](field@KvParser::span) always points to the top of derive macro (`#[derive(GodotClass)]`).
    pub fn handle_expr_with_key(&mut self, key: &str) -> ParseResult<Option<(Ident, TokenStream)>> {
        match self.map.remove_entry(&ident(key)) {
            None => Ok(None),
            // The `key` that was removed from the map has the correct span.
            Some((key, value)) => match value {
                None => bail!(key, "expected `{key}` to be followed by `= expression`"),
                Some(value) => Ok(Some((key, value.expr()?))),
            },
        }
    }

    /// Shortcut for [KvParser::handle_expr_with_key] which returns only the value.
    pub fn handle_expr(&mut self, key: &str) -> ParseResult<Option<TokenStream>> {
        match self.handle_expr_with_key(key)? {
            Some((_key, value)) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    pub fn handle_literal(
        &mut self,
        key: &str,
        expected_type: &str,
    ) -> ParseResult<Option<Literal>> {
        let Some((key, expr)) = self.handle_expr_with_key(key)? else {
            return Ok(None);
        };

        let mut tokens = expr.into_iter();
        let Some(TokenTree::Literal(lit)) = tokens.next() else {
            return bail!(
                key,
                "missing value for '{key}' (must be {expected_type} literal)"
            );
        };

        if let Some(surplus) = tokens.next() {
            return bail!(
                key,
                "value for '{key}' must be {expected_type} literal; found extra {surplus:?}"
            );
        }
        Ok(Some(lit))
    }

    pub fn handle_usize(&mut self, key: &str) -> ParseResult<Option<usize>> {
        let Some(lit) = self.handle_literal(key, "unsigned integer")? else {
            return Ok(None);
        };

        let Ok(int) = lit.to_string().parse() else {
            return bail!(
                key,
                "value for '{key}' must be unsigned integer literal; found {lit:?}"
            );
        };

        Ok(Some(int))
    }

    #[allow(dead_code)]
    pub fn handle_bool(&mut self, key: &str) -> ParseResult<Option<bool>> {
        let Some((key, expr)) = self.handle_expr_with_key(key)? else {
            return Ok(None);
        };

        let mut tokens = expr.into_iter();
        let Some(TokenTree::Ident(id)) = tokens.next() else {
            return bail!(key, "missing value for '{key}' (must be bool literal)");
        };

        if let Some(surplus) = tokens.next() {
            return bail!(
                key,
                "value for '{key}' must be bool literal; found extra {surplus:?}"
            );
        }

        let Ok(b) = id.to_string().parse() else {
            return bail!(key, "value for '{key}' must be bool literal; found {id:?}");
        };

        Ok(Some(b))
    }

    /// Handles a key that must be provided and must have an identifier as the value.
    pub fn handle_ident_required(&mut self, key: &str) -> ParseResult<Ident> {
        self.handle_ident(key)?
            .ok_or_else(|| error!(self.span, "missing required argument `{key} = identifier`",))
    }

    /// Handles a key that must be provided and must have a value.
    pub fn handle_expr_required(&mut self, key: &str) -> ParseResult<TokenStream> {
        self.handle_expr(key)?
            .ok_or_else(|| error!(self.span, "missing required argument `{key} = expression`",))
    }

    /// Explicit "pre-destructor" that must be called, and checks that all map entries have been
    /// consumed.
    // We used to check in a `Drop` impl that `finish` has actually been called, but that turns out
    // to be overzealous: it panics if the calling function just wants to return an error and drops
    // a partially-consumed parser.
    pub fn finish(self) -> ParseResult<()> {
        if self.map.is_empty() {
            Ok(())
        } else {
            let errors = self
                .map
                .keys()
                .map(|ident| error!(ident, "unrecognized key `{ident}`"));
            Err(errors
                .reduce(|mut a, b| {
                    a.combine(b);
                    a
                })
                .unwrap())
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct KvValue {
    /// Tokens comprising this value. Guaranteed to be nonempty.
    tokens: Vec<TokenTree>,
}

impl KvValue {
    pub(super) fn new(tokens: Vec<TokenTree>) -> Self {
        assert!(!tokens.is_empty());
        Self { tokens }
    }

    pub fn into_tokens(self) -> Vec<TokenTree> {
        self.tokens
    }

    pub fn expr(self) -> ParseResult<TokenStream> {
        Ok(self.tokens.into_iter().collect())
    }

    pub fn single(mut self) -> ParseResult<TokenTree> {
        if self.tokens.len() > 1 {
            return bail!(&self.tokens[1], "expected a single item");
        }

        Ok(self.tokens.remove(0))
    }

    pub fn ident(self) -> ParseResult<Ident> {
        match self.single()? {
            TokenTree::Ident(ident) => Ok(ident),
            tt => {
                bail!(tt, "expected identifier")
            }
        }
    }

    pub fn as_key_value(&self) -> ParseResult<(Ident, Self)> {
        if self.tokens.len() < 3 {
            return bail!(&self.tokens[0], "expected `key = expression`");
        }

        let key = match &self.tokens[0] {
            TokenTree::Ident(id) => id.clone(),
            other => return bail!(other, "expected identifier"),
        };

        let has_equals = match &self.tokens[1] {
            TokenTree::Punct(punct) => punct.as_char() == '=' && punct.spacing() == Spacing::Alone,
            _ => false,
        };

        if !has_equals {
            return bail!(&self.tokens[1], "expected `=`");
        }

        Ok((key, Self::new(self.tokens[2..].into())))
    }

    pub fn as_ident(&self) -> ParseResult<Ident> {
        if self.tokens.len() > 1 {
            return bail!(&self.tokens[1], "expected a single identifier");
        }

        match &self.tokens[0] {
            TokenTree::Ident(id) => Ok(id.clone()),
            other => bail!(other, "expected identifier"),
        }
    }

    pub fn as_literal(&self) -> ParseResult<Literal> {
        if self.tokens.len() > 1 {
            return bail!(&self.tokens[1], "expected a single literal");
        }

        match &self.tokens[0] {
            TokenTree::Literal(literal) => Ok(literal.clone()),
            other => bail!(other, "expected literal"),
        }
    }
}

struct ParserState<'a> {
    attr_name: String,
    tokens: std::slice::Iter<'a, TokenTree>,
    prev: Option<&'a TokenTree>,
    cur: Option<&'a TokenTree>,
}

impl<'a> ParserState<'a> {
    pub fn parse(attr_name: String, attr_value: &'a venial::AttributeValue) -> ParseResult<KvMap> {
        let mut tokens = match attr_value {
            venial::AttributeValue::Equals(punct, _tokens) => {
                return bail!(punct, "expected `(` or `]`");
            }
            _ => attr_value.get_value_tokens().iter(),
        };
        let cur = tokens.next();

        let parser = Self {
            attr_name,
            tokens,
            prev: None,
            cur,
        };

        parser.parse_map()
    }

    fn parse_map(mut self) -> ParseResult<KvMap> {
        let mut map: KvMap = HashMap::new();
        // Whether the previous expression might be missing parentheses. Used only for hints in
        // error reporting.
        let mut prev_expr_complex = false;

        while let Some(cur) = self.cur {
            match cur {
                TokenTree::Ident(key) => {
                    self.next();
                    let value = self.parse_opt_value(key, prev_expr_complex)?;
                    if map.contains_key(key) {
                        return bail!(key, "duplicate key `{key}`");
                    }
                    prev_expr_complex = match &value {
                        None => false,
                        Some(value) => value.tokens.len() > 1,
                    };
                    map.insert(key.clone(), value);
                }
                _ => {
                    let parens_hint = if prev_expr_complex {
                        let attr = &self.attr_name;
                        format!("\nnote: the preceding `,` is interpreted as a separator between arguments to `#[{attr}]`; if you meant the `,` as part of an expression, surround the expression with parentheses")
                    } else {
                        "".to_owned()
                    };
                    return bail!(cur, "expected identifier{parens_hint}");
                }
            }
        }

        Ok(map)
    }

    fn parse_opt_value(
        &mut self,
        key: &Ident,
        prev_expr_complex: bool,
    ) -> ParseResult<Option<KvValue>> {
        let value = match self.cur {
            // End of input directly after a key
            None => None,
            // Comma following key
            Some(tt) if is_punct(tt, ',') => {
                self.next();
                None
            }
            // Equals sign following key
            Some(tt) if is_punct(tt, '=') => {
                self.next();
                Some(self.parse_value()?)
            }
            Some(tt) => {
                let parens_hint = if prev_expr_complex {
                    let attr = &self.attr_name;
                    format!("\nnote: `{key}` is interpreted as the next argument to `#[{attr}]`; if you meant it as part of an expression, surround the expression with parentheses")
                } else {
                    "".to_owned()
                };
                return bail!(
                    tt,
                    "expected next argument, or `= value` following `{key}`{parens_hint}"
                );
            }
        };
        Ok(value)
    }

    fn parse_value(&mut self) -> ParseResult<KvValue> {
        let mut tokens = Vec::new();
        while let Some(cur) = self.cur {
            if is_punct(cur, ',') {
                self.next();
                break;
            }
            tokens.push(cur.clone());
            self.next();
        }
        if tokens.is_empty() {
            // `cur` might be `None` at this point, so we point at the previous token instead.
            // This could be the `=` sign or a `,` directly after `=`.
            return bail!(self.prev.unwrap(), "expected value after `=`");
        }
        Ok(KvValue::new(tokens))
    }

    fn next(&mut self) {
        self.prev = self.cur;
        self.cur = self.tokens.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use quote::quote;

    /// A quick and dirty way to compare two expressions for equality. Only for unit tests; not
    /// very suitable for production code.
    impl PartialEq for KvValue {
        fn eq(&self, other: &Self) -> bool {
            let to_strings = |kv: &Self| {
                kv.tokens
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            };
            to_strings(self) == to_strings(other)
        }
    }

    macro_rules! kv_map {
        (
            $($key:ident => $value:expr),*
            $(,)?
        ) => {
            {
                let mut map = std::collections::HashMap::new();
                $(
                    map.insert(ident(stringify!($key)), $value);
                )*
                map
            }
        };
    }

    macro_rules! kv_value {
        ($($args:tt)*) => {
            KvValue::new(quote! { $($args)* }.into_iter().collect())
        }
    }

    fn parse(input_tokens: TokenStream) -> KvMap {
        let input = quote! {
            #input_tokens
            fn func();
        };
        let decl = venial::parse_item(input);

        let attrs = &decl
            .as_ref()
            .expect("decl")
            .as_function()
            .expect("fn")
            .attributes;

        assert_eq!(attrs.len(), 1);
        let attr_value = &attrs[0].value;
        ParserState::parse("attr".to_owned(), attr_value).expect("parse")
    }

    fn expect_parsed(input_tokens: TokenStream, output_map: KvMap) {
        let mut parsed = parse(input_tokens);

        for (key, value) in output_map {
            assert_eq!(
                parsed.remove(&key),
                Some(value),
                "incorrect parsed value for `{key}`"
            );
        }

        assert!(parsed.is_empty(), "Remaining entries in map");
    }

    #[test]
    fn test_parse_kv_just_key() {
        expect_parsed(
            quote! {
                #[attr(just_key)]
            },
            kv_map!(
                just_key => None,
            ),
        );
    }

    #[test]
    fn test_parse_kv_ident() {
        expect_parsed(
            quote! {
                #[attr(key = value)]
            },
            kv_map!(
                key => Some(kv_value!(value)),
            ),
        );
    }

    #[test]
    fn test_parse_kv_trailing_comma() {
        expect_parsed(
            quote! {
                #[attr(key = value,)]
            },
            kv_map!(
                key => Some(kv_value!(value)),
            ),
        );
    }

    #[test]
    fn test_parse_kv_first_last_expr() {
        expect_parsed(
            quote! {
                #[attr(first = foo, middle = bar, last = qux)]
            },
            kv_map!(
                first => Some(kv_value!(foo)),
                middle => Some(kv_value!(bar)),
                last => Some(kv_value!(qux)),
            ),
        );
    }

    #[test]
    fn test_parse_kv_first_last_alone() {
        expect_parsed(
            quote! {
                #[attr(first, middle = bar, last)]
            },
            kv_map!(
                first => None,
                middle => Some(kv_value!(bar)),
                last => None,
            ),
        );
    }

    #[test]
    fn test_parse_kv_exprs() {
        expect_parsed(
            quote! {
                #[attr(
                    pos = 42,
                    neg = -42,
                    str_lit = "string",
                    sum = 1 + 1,
                    vec = Vector2::new(1.0, -1.0e2),
                    // Currently needs parentheses.
                    generic = (HashMap::<String, Vec<usize>>::new()),
                    // Currently needs parentheses.
                    closure = (|a: &u32, b: &u32| a + b),
                )]
            },
            kv_map!(
                pos => Some(kv_value!(42)),
                neg => Some(kv_value!(-42)),
                str_lit => Some(kv_value!("string")),
                sum => Some(kv_value!(1 + 1)),
                vec => Some(kv_value!(Vector2::new(1.0, -1.0e2))),
                generic => Some(kv_value!((HashMap::<String, Vec<usize>>::new()))),
                closure => Some(kv_value!((|a: &u32, b: &u32| a + b))),
            ),
        );
    }
}
