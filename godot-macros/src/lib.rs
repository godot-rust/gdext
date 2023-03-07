/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Extension API for Godot classes, used with `#[godot_api]`.
///
/// Helps with adding custom functionality:
/// * `init` constructors
/// * `to_string` method
/// * Custom register methods (builder style)
/// * All the lifecycle methods like `ready`, `process` etc.
///
/// These trait are special in that it needs to be used in combination with the `#[godot_api]`
/// proc-macro attribute to ensure proper registration of its methods. All methods have
/// default implementations, so you can select precisely which functionality you want to have.
/// Those default implementations are never called however, the proc-macro detects what you implement.
///
/// Do not call any of these methods directly -- they are an interface to Godot. Functionality
/// described here is available through other means (e.g. `init` via `Gd::new_default`).
/// It is not enough to impl `GodotExt` to be registered in Godot, for this you should look at
/// [ExtensionLibrary](crate::init::ExtensionLibrary).
///
/// If you wish to create a struct deriving GodotClass, you should impl the trait <Base>Virtual,
/// for your desired Base (i.e. `RefCountedVirtual`, `NodeVirtual`).
///
/// # Examples
///
/// ## Example with `RefCounted` as a base
///
/// ```
///# use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyRef;
///
/// #[godot_api]
/// impl MyRef {
///     #[func]
///     pub fn hello_world(&mut self) {
///         godot_print!("Hello World!")
///     }
/// }
///
/// #[godot_api]
/// impl RefCountedVirtual for MyRef {
///     fn init(_: Base<RefCounted>) -> Self {
///         MyRef
///     }
/// }
/// ```
///
/// The following example allows to use MyStruct in GDScript for instance by calling
/// `MyStruct.new().hello_world()`.
///
///
/// Note that you have to implement init otherwise you won't be able to call new or any
/// other methods from GDScript.
///
/// ## Example with `Node` as a Base
///
/// ```
///# use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(base=Node)]
/// pub struct MyNode {
///     #[base]
///     base: Base<Node>,
/// }
///
/// #[godot_api]
/// impl NodeVirtual for MyNode {
///     fn init(base: Base<Node>) -> Self {
///         MyNode { base }
///     }
///     fn ready(&mut self) {
///         godot_print!("Hello World!");
///     }
/// }
/// ```
///
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
