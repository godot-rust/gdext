/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::ident;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use venial::Declaration;

mod derive_godot_class;
mod gdextension;
mod godot_api;
mod itest;
mod util;

#[proc_macro_derive(GodotClass, attributes(class, property, export, base, signal))]
pub fn derive_native_class(input: TokenStream) -> TokenStream {
    translate(input, derive_godot_class::transform)
}

/// Attribute to annotate `impl` blocks.
#[doc(alias = "GodotExt")]
#[proc_macro_attribute]
pub fn godot_api(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, godot_api::transform)
}

/// Similar to `#[test]`, but runs an integration test with Godot.
///
/// Transforms the `fn` into one returning `bool` (success of the test), which must be called explicitly.
#[proc_macro_attribute]
pub fn itest(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate_meta("itest", meta, input, itest::transform)
}

/// Proc-macro attribute to be used in combination with the [`ExtensionLibrary`] trait.
///
/// [`ExtensionLibrary`]: crate::init::ExtensionLibrary
// FIXME intra-doc link
#[proc_macro_attribute]
pub fn gdextension(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate_meta("gdextension", meta, input, gdextension::transform)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

type ParseResult<T> = Result<T, venial::Error>;

fn translate<F>(input: TokenStream, transform: F) -> TokenStream
where
    F: FnOnce(Declaration) -> ParseResult<TokenStream2>,
{
    let input2 = TokenStream2::from(input);

    let result2 = venial::parse_declaration(input2)
        .and_then(transform)
        .unwrap_or_else(|e| e.to_compile_error());

    TokenStream::from(result2)
}

fn translate_meta<F>(
    self_name: &str,
    meta: TokenStream,
    input: TokenStream,
    transform: F,
) -> TokenStream
where
    F: FnOnce(Declaration) -> ParseResult<TokenStream2>,
{
    let self_name = ident(self_name);
    let input2 = TokenStream2::from(input);
    let meta2 = TokenStream2::from(meta);

    // Hack because venial doesn't support direct meta parsing yet
    let input = quote! {
        #[#self_name(#meta2)]
        #input2
    };

    let result2 = venial::parse_declaration(input)
        .and_then(transform)
        .unwrap_or_else(|e| e.to_compile_error());

    TokenStream::from(result2)
}
