/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

mod derive_godot_class;
mod gdextension;
mod godot_api;
mod itest;
mod util;

#[proc_macro_derive(GodotClass, attributes(class, property, export, base, signal))]
pub fn derive_native_class(input: TokenStream) -> TokenStream {
    translate(input, derive_godot_class::transform)
}

#[proc_macro_attribute]
pub fn godot_api(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, godot_api::transform)
}

/// Similar to `#[test]`, but runs an integration test with Godot.
///
/// Transforms the `fn` into one returning `bool` (success of the test), which must be called explicitly.
#[proc_macro_attribute]
pub fn itest(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, itest::transform)
}

/// Proc-macro attribute to be used in combination with the [`ExtensionLibrary`] trait.
///
/// [`ExtensionLibrary`]: crate::init::ExtensionLibrary
// FIXME intra-doc link
#[proc_macro_attribute]
pub fn gdextension(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate_meta(meta, input, gdextension::transform)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

type ParseResult<T> = Result<T, venial::Error>;

fn translate<F>(input: TokenStream, transform: F) -> TokenStream
where
    F: FnOnce(TokenStream2) -> ParseResult<TokenStream2>,
{
    let input2 = TokenStream2::from(input);
    let result2: TokenStream2 = match transform(input2) {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    };
    TokenStream::from(result2)
}

fn translate_meta<F>(meta: TokenStream, input: TokenStream, transform: F) -> TokenStream
where
    F: FnOnce(TokenStream2, TokenStream2) -> ParseResult<TokenStream2>,
{
    let input2 = TokenStream2::from(input);
    let meta2 = TokenStream2::from(meta);
    let result2: TokenStream2 = match transform(meta2, input2) {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    };
    TokenStream::from(result2)
}
