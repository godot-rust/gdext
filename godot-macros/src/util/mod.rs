/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with codegen crate

use crate::ParseResult;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::spanned::Spanned;
use quote::{format_ident, ToTokens};
use std::collections::HashMap;
use venial::{Attribute, Error, Function, GenericParamList, Impl, WhereClause};

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn bail<R, T>(msg: impl AsRef<str>, tokens: T) -> ParseResult<R>
where
    T: Spanned,
{
    Err(error(msg, tokens))
}

pub fn error<T>(msg: impl AsRef<str>, tokens: T) -> Error
where
    T: Spanned,
{
    Error::new_at_span(tokens.__span(), msg.as_ref())
}

pub fn reduce_to_signature(function: &Function) -> Function {
    let mut reduced = function.clone();
    reduced.vis_marker = None; // TODO needed?
    reduced.attributes.clear();
    reduced.tk_semicolon = None;
    reduced.body = None;

    reduced
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Key-value parsing of proc attributes

#[derive(Clone, Debug)]
pub(crate) struct KvValue {
    /// Tokens comprising this value. Guaranteed to be nonempty.
    tokens: Vec<TokenTree>,
}

impl KvValue {
    fn new(tokens: Vec<TokenTree>) -> Self {
        assert!(!tokens.is_empty());
        Self { tokens }
    }

    pub fn expr(self) -> ParseResult<TokenStream> {
        Ok(self.tokens.into_iter().collect())
    }

    pub fn ident(self) -> ParseResult<Ident> {
        let ident = match &self.tokens[0] {
            TokenTree::Ident(ident) => ident.clone(),
            tt => {
                return bail("expected identifier", tt);
            }
        };
        if self.tokens.len() > 1 {
            return bail(
                "expected a single identifier, not an expression",
                &self.tokens[1],
            );
        }
        Ok(ident)
    }
}

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
        attributes: &[Attribute],
        expected: &str,
        context: impl ToTokens,
    ) -> ParseResult<Self> {
        match Self::parse(attributes, expected) {
            Ok(Some(result)) => Ok(result),
            Ok(None) => bail(
                format!("expected attribute #[{expected}], but not present"),
                context,
            ),
            Err(e) => Err(e),
        }
    }

    /// Create a new parser which checks for presence of an `#[expected]` attribute.
    pub fn parse(attributes: &[Attribute], expected: &str) -> ParseResult<Option<Self>> {
        let mut found_attr: Option<Self> = None;

        for attr in attributes.iter() {
            let path = &attr.path;
            if path_is_single(path, expected) {
                if found_attr.is_some() {
                    return bail(
                        format!("only a single #[{expected}] attribute allowed"),
                        attr,
                    );
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

    pub fn span(&self) -> Span {
        self.span
    }

    /// - For missing keys, returns `None`.
    /// - For a key with no value, returns `Some(None)`.
    /// - For a key with a value, returns `Some(value)`.
    pub fn handle_any(&mut self, key: &str) -> Option<Option<KvValue>> {
        self.map.remove(&ident(key))
    }

    /// Handles a key that can only occur without a value, e.g. `#[attr(toggle)]`. Returns whether
    /// the key is present.
    pub fn handle_alone(&mut self, key: &str) -> ParseResult<bool> {
        match self.handle_any(key) {
            None => Ok(false),
            Some(value) => match value {
                None => Ok(true),
                Some(value) => bail(
                    format!("key `{key}` should not have a value"),
                    &value.tokens[0],
                ),
            },
        }
    }

    /// Handles an optional key that can only occur with an identifier as the value.
    pub fn handle_ident(&mut self, key: &str) -> ParseResult<Option<Ident>> {
        match self.map.remove_entry(&ident(key)) {
            None => Ok(None),
            // The `key` that was removed from the map has the correct span.
            Some((key, value)) => match value {
                None => bail(
                    format!("expected `{key}` to be followed by `= identifier`"),
                    key,
                ),
                Some(value) => Ok(Some(value.ident()?)),
            },
        }
    }

    /// Handles an optional key that can occur with arbitrary tokens as the value.
    pub fn handle_expr(&mut self, key: &str) -> ParseResult<Option<TokenStream>> {
        match self.map.remove_entry(&ident(key)) {
            None => Ok(None),
            // The `key` that was removed from the map has the correct span.
            Some((key, value)) => match value {
                None => bail(
                    format!("expected `{key}` to be followed by `= expression`"),
                    key,
                ),
                Some(value) => Ok(Some(value.expr()?)),
            },
        }
    }

    /// Handles a key that must be provided and must have an identifier as the value.
    pub fn handle_ident_required(&mut self, key: &str) -> ParseResult<Ident> {
        self.handle_ident(key)?.ok_or_else(|| {
            error(
                format!("missing required argument `{key} = identifier`"),
                self.span,
            )
        })
    }

    /// Handles a key that must be provided and must have a value.
    pub fn handle_expr_required(&mut self, key: &str) -> ParseResult<TokenStream> {
        self.handle_expr(key)?.ok_or_else(|| {
            error(
                format!("missing required argument `{key} = expression`"),
                self.span,
            )
        })
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
                .map(|ident| error(format!("unrecognized key `{ident}`"), ident));
            Err(errors
                .reduce(|mut a, b| {
                    a.combine(b);
                    a
                })
                .unwrap())
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
                return bail("expected `(` or `]`", punct);
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
                        return bail(format!("duplicate key `{key}`"), key);
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
                    return bail(format!("expected identifier{parens_hint}"), cur);
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
                return bail(
                    format!("expected next argument, or `= value` following `{key}`{parens_hint}"),
                    tt,
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
            return bail("expected value after `=`", self.prev.unwrap());
        }
        Ok(KvValue::new(tokens))
    }

    fn next(&mut self) {
        self.prev = self.cur;
        self.cur = self.tokens.next();
    }
}

fn is_punct(tt: &TokenTree, c: char) -> bool {
    match tt {
        TokenTree::Punct(punct) => punct.as_char() == c,
        _ => false,
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Validation for trait/impl

/// Given an impl block for a trait, returns whether that is an impl
/// for a trait with the given name.
///
/// That is, if `name` is `"MyTrait"`, then this function returns true
/// if and only if `original_impl` is a declaration of the form `impl
/// MyTrait for SomeType`. The type `SomeType` is irrelevant in this
/// example.
pub(crate) fn is_impl_named(original_impl: &Impl, name: &str) -> bool {
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    extract_typename(trait_name).map_or(false, |seg| seg.ident == name)
}

/// Validates either:
/// a) the declaration is `impl Trait for SomeType`, if `expected_trait` is `Some("Trait")`
/// b) the declaration is `impl SomeType`, if `expected_trait` is `None`
pub(crate) fn validate_impl(
    original_impl: &Impl,
    expected_trait: Option<&str>,
    attr: &str,
) -> ParseResult<Ident> {
    if let Some(expected_trait) = expected_trait {
        // impl Trait for Self -- validate Trait
        if !is_impl_named(original_impl, expected_trait) {
            return bail(
                format!("#[{attr}] for trait impls requires trait to be `{expected_trait}`"),
                original_impl,
            );
        }
    }

    // impl Trait for Self -- validate Self
    validate_self(original_impl, attr)
}

/// Validates that the declaration is the of the form `impl Trait for
/// SomeType`, where the name of `Trait` ends in `Virtual`.
pub(crate) fn validate_trait_impl_virtual(
    original_impl: &Impl,
    attr: &str,
) -> ParseResult<(Ident, Ident)> {
    let trait_name = original_impl.trait_ty.as_ref().unwrap(); // unwrap: already checked outside
    let typename = extract_typename(trait_name);

    // Validate trait
    if !typename
        .as_ref()
        .map_or(false, |seg| seg.ident.to_string().ends_with("Virtual"))
    {
        return bail(
            format!("#[{attr}] for trait impls requires a virtual method trait (trait name should end in 'Virtual')"),
            original_impl,
        );
    }

    // Validate self
    validate_self(original_impl, attr).map(|class_name| {
        let trait_name = typename.unwrap(); // unwrap: already checked in 'Validate trait'
        (class_name, trait_name.ident)
    })
}

fn validate_self(original_impl: &Impl, attr: &str) -> ParseResult<Ident> {
    if let Some(segment) = extract_typename(&original_impl.self_ty) {
        if segment.generic_args.is_none() {
            Ok(segment.ident)
        } else {
            bail(
                format!("#[{attr}] for does currently not support generic arguments"),
                original_impl,
            )
        }
    } else {
        bail(
            format!("#[{attr}] requires Self type to be a simple path"),
            original_impl,
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

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
            KvValue::new(quote!($($args)*).into_iter().collect())
        }
    }

    fn parse(input_tokens: TokenStream) -> KvMap {
        let input = quote! {
            #input_tokens
            fn func();
        };
        let decl = venial::parse_declaration(input);

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

pub(crate) fn path_is_single(path: &[TokenTree], expected: &str) -> bool {
    path.len() == 1 && path[0].to_string() == expected
}

pub(crate) fn path_ends_with(path: &[TokenTree], expected: &str) -> bool {
    // could also use TyExpr::as_path()
    path.last()
        .map(|last| last.to_string() == expected)
        .unwrap_or(false)
}

pub(crate) struct DeclInfo {
    pub where_: Option<WhereClause>,
    pub generic_params: Option<GenericParamList>,
    pub name: proc_macro2::Ident,
    pub name_string: String,
}

pub(crate) fn decl_get_info(decl: &venial::Declaration) -> DeclInfo {
    let (where_, generic_params, name, name_string) =
        if let venial::Declaration::Struct(struct_) = decl {
            (
                struct_.where_clause.clone(),
                struct_.generic_params.clone(),
                struct_.name.clone(),
                struct_.name.to_string(),
            )
        } else if let venial::Declaration::Enum(enum_) = decl {
            (
                enum_.where_clause.clone(),
                enum_.generic_params.clone(),
                enum_.name.clone(),
                enum_.name.to_string(),
            )
        } else {
            panic!("only enums and structs are supported at the moment.")
        };
    DeclInfo {
        where_,
        generic_params,
        name,
        name_string,
    }
}
