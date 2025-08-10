/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: some code duplication with codegen crate

use std::collections::VecDeque;

use proc_macro2::{Delimiter, Ident, Span, TokenStream, TokenTree};

use crate::util::kv_parser::KvValue;
use crate::util::{bail, delimiter_opening_char, is_punct, KvParser};
use crate::ParseResult;

/// Parses a list of tokens as an ordered list of values. Unlike [`KvParser`] which treats the tokens as a
/// set of values.
pub struct ListParser {
    lists: VecDeque<KvValue>,
    /// The last span of the list, usually a closing parenthesis.
    span_close: Span,
}

impl ListParser {
    /// Create a new list parser from a `key = (elem1, elem2, ...)` attribute.
    ///
    /// The value is optional, and an attribute without a value will be treated as having an empty list.
    pub(crate) fn new_from_kv(
        parser: &mut KvParser,
        key: &str,
        delimiter: Delimiter,
    ) -> ParseResult<Option<Self>> {
        let mut tokens = match parser.handle_any_entry(key) {
            // No key -> missing
            None => return Ok(None),
            // Key without list -> exists
            Some((key, None)) => {
                return Ok(Some(Self {
                    lists: VecDeque::new(),
                    span_close: key.span(),
                }))
            }
            // Key with list -> exists, must check list format
            Some((_, Some(tokens))) => tokens.into_tokens(),
        };

        if tokens.len() > 1 {
            return bail!(&tokens[1], "unexpected expression");
        }

        Ok(Some(Self::new_from_tree(tokens.remove(0), delimiter)?))
    }

    /// Create a new parser from the given tokentree.
    ///
    /// Ensures that the tree is a list of lists of tokentrees, delimited by the provided delimiter, where
    /// no sublist is empty. Except for the last list, which is allowed to be empty to allow for trailing
    /// commas.
    pub fn new_from_tree(tree: TokenTree, delimiter: Delimiter) -> ParseResult<Self> {
        let group = match tree {
            TokenTree::Group(group) => group,
            _ => return bail!(tree, "expected list of items"),
        };

        if group.delimiter() != delimiter {
            let expected = delimiter_opening_char(delimiter);
            let got = delimiter_opening_char(group.delimiter());

            return bail!(group.span_open(), "expected `{expected}`, got `{got}`");
        }

        let trees: Vec<TokenTree> = group.stream().into_iter().collect();

        let raw_lists = trees
            .split_inclusive(|tree| is_punct(tree, ','))
            .collect::<Vec<_>>();

        let list_len = raw_lists.len();
        let mut lists = Vec::new();

        for (i, list) in raw_lists.into_iter().enumerate() {
            let is_last = i == list_len - 1;

            // every list except the last one must contain at least a comma so this means we have a trailing
            // comma.
            if list.is_empty() {
                break;
            }

            // does list only contain `,`?
            if !is_last && list.len() == 1 {
                let list_stream = list.iter().cloned().collect::<TokenStream>();
                return bail!(list_stream, "expected expression, found `,`");
            }

            let end = if is_last { list.len() } else { list.len() - 1 };

            lists.push(KvValue::new((&list[..end]).into()));
        }

        Ok(Self {
            lists: lists.into(),
            span_close: group.span_close(),
        })
    }

    /// Get the next element from the list, starting at the front.
    fn pop_next(&mut self) -> Option<KvValue> {
        self.lists.pop_front()
    }

    pub(crate) fn peek(&self) -> Option<&KvValue> {
        self.lists.front()
    }

    /// Take the next element of the list, ensuring it is an expression.
    pub(crate) fn next_expr(&mut self) -> ParseResult<TokenStream> {
        match self.pop_next() {
            Some(kv) => kv.expr(),
            None => bail!(self.span_close, "expected expression"),
        }
    }

    /// Take the next element of the list unconditionally,
    /// and returns an error if it is not an identifier.
    ///
    /// Returns `Ok(None)` if there are no more elements left.
    pub(crate) fn next_ident(&mut self) -> ParseResult<Option<Ident>> {
        self.pop_next().map(|kv| kv.ident()).transpose()
    }

    /// Take the next element of the list, if it is an identifier.
    ///
    /// Returns `Err` if the next element isn't an identifier,
    /// or `Ok(None)` if there are no more elements left
    pub fn try_next_ident(&mut self) -> ParseResult<Option<Ident>> {
        let Some(kv) = self.peek() else {
            return Ok(None);
        };

        let id = kv.as_ident()?;

        _ = self.pop_next();

        Ok(Some(id))
    }

    /// Checks to see if there is a next element, and if so,
    /// whether it is one of the allowed identifiers,
    /// returning `Ok(Some(ident))` if successful.
    ///
    /// Returns `Err(e)` if the next identifier is not in the allowed list,
    /// but does not consume it.
    ///
    /// Returns `Ok(None)` if there are no more elements left.
    pub fn next_allowed_ident(&mut self, allowed_ids: &[&str]) -> ParseResult<Option<Ident>> {
        let Some(next_id) = self.try_next_ident()? else {
            return Ok(None);
        };

        for id in allowed_ids {
            if next_id == id {
                return Ok(Some(next_id));
            }
        }

        // None of the allowed identifiers matched, so we return an error
        let allowed_values = allowed_ids.join(",");
        bail!(next_id, "expected one of: \"{allowed_values}\"")
    }

    /// Take the next element of the list, if it is a key-value pair of the form `key = expression`.
    pub(crate) fn try_next_key_value(&mut self) -> Option<(Ident, KvValue)> {
        let kv = self.peek()?;

        if let Ok((key, value)) = kv.as_key_value() {
            // If peek() parsed successfully, we consume the next element
            _ = self.pop_next();

            Some((key, value))
        } else {
            None
        }
    }

    /// Take the next element of the list, ensuring it is either a single identifier or a key-value pair of
    /// the form `key = expression`.
    pub(crate) fn next_key_optional_value(
        &mut self,
    ) -> ParseResult<Option<(Ident, Option<KvValue>)>> {
        if let Some((key, value)) = self.try_next_key_value() {
            return Ok(Some((key, Some(value))));
        }

        match self.try_next_ident() {
            Ok(opt) => Ok(opt.map(|k| (k, None))),
            Err(err) => bail!(err.span(), "expected `key [= value]`"),
        }
    }

    /// Like `next_key_optional_value`, but checks if input flags and keys are in the allowed sets and `Err`s if not.
    ///
    /// If an allowed flag appears as a key or an allowed key as a flag, that will also `Err` with a helpful message.
    pub(crate) fn next_allowed_key_optional_value(
        &mut self,
        allowed_flag_keys: &[&str],
        allowed_kv_keys: &[&str],
    ) -> ParseResult<Option<(Ident, Option<KvValue>)>> {
        let allowed_keys = || {
            let allowed_flag_keys = allowed_flag_keys.join(",");
            let allowed_kv_keys = allowed_kv_keys.join(",");
            [allowed_flag_keys, allowed_kv_keys].join(",")
        };
        match self.next_key_optional_value()? {
            Some((key, None)) if !allowed_flag_keys.contains(&key.to_string().as_str()) => {
                if allowed_kv_keys.contains(&key.to_string().as_str()) {
                    return bail!(key, "`{key}` requires a value `{key} = VALUE`");
                }
                bail!(key, "expected one of \"{}\"", allowed_keys())
            }
            Some((key, Some(_))) if !allowed_kv_keys.contains(&key.to_string().as_str()) => {
                if allowed_flag_keys.contains(&key.to_string().as_str()) {
                    return bail!(key, "key `{key}` mustn't have a value");
                }
                bail!(key, "expected one of \"{}\"", allowed_keys())
            }
            key_maybe_value => Ok(key_maybe_value),
        }
    }

    /// Ensure all values have been consumed.
    pub fn finish(&mut self) -> ParseResult<()> {
        if let Some(kv) = self.pop_next() {
            let stream: TokenStream = kv.expr()?;

            return bail!(&stream, "unrecognized value `{stream}`");
        }

        Ok(())
    }
}
