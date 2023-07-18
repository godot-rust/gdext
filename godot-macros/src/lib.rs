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

mod derive_from_variant;
mod derive_godot_class;
mod derive_to_variant;
mod gdextension;
mod godot_api;
mod itest;
mod method_registration;
mod util;

// Below intra-doc link to the trait only works as HTML, not as symbol link.
/// Derive macro for [the `GodotClass` trait](../obj/trait.GodotClass.html) on structs. You must use this
/// macro; manual implementations of the `GodotClass` trait are not supported.
///
/// # Construction
///
/// To generate a constructor that will let you call `MyStruct.new()` from GDScript, annotate your
/// struct with `#[class(init)]`:
///
/// ```
/// # use godot_macros::GodotClass;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyStruct {
///     // ...
/// }
/// ```
///
/// The generated `init` function will initialize each struct field (except the field annotated
/// with `#[base]`, if any) using `Default::default()`. To assign some other value, annotate the
/// field with `#[init(default = ...)]`:
///
/// ```
/// # use godot_macros::GodotClass;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyStruct {
///     #[init(default = 42)]
///     my_field: i64
/// }
/// ```
///
/// The given value can be any Rust expression that can be evaluated in the scope where you write
/// the attribute. However, due to limitations in the parser, some complex expressions must be
/// surrounded by parentheses. This is the case if the expression includes a `,` that is _not_
/// inside any pair of `(...)`, `[...]` or `{...}` (even if it is, for example, inside `<...>` or
/// `|...|`). A contrived example:
///
/// ```
/// # use godot_macros::GodotClass;
/// # use std::collections::HashMap;
/// # #[derive(GodotClass)]
/// # #[class(init)]
/// # struct MyStruct {
///     #[init(default = (HashMap::<i64, i64>::new()))]
///     //                             ^ parentheses needed due to this comma
/// #   my_field: HashMap<i64, i64>,
/// # }
/// ```
///
/// # Inheritance
///
/// Unlike C++, Rust doesn't really have inheritance, but the GDExtension API lets us "inherit"
/// from a built-in engine class.
///
/// By default, classes created with this library inherit from `RefCounted`.
///
/// To specify a different class to inherit from, add `#[class(base = Base)]` as an annotation on
/// your `struct`:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(base = Node2D)]
/// struct MyStruct {
///     // ...
/// }
/// ```
///
/// If you need a reference to the base class, you can add a field of type `Gd<Base>` and annotate
/// it with `#[base]`:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(base = Node2D)]
/// struct MyStruct {
///     #[base]
///     base: Gd<Node2D>,
/// }
/// ```
///
/// # Exported properties
///
/// In GDScript, there is a distinction between
/// [properties](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html#properties-setters-and-getters)
/// (fields with a `get` or `set` declaration) and
/// [exports](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_exports.html)
/// (fields annotated with `@export`). In the GDExtension API, these two concepts are represented with
/// `#[var]` and `#[export]` attributes respectively.
///
/// To create a property, you can use the `#[var]` annotation:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     #[var]
///     my_field: i64,
/// }
///
/// #[godot_api]
/// impl MyStruct {}
/// ```
///
/// This makes the field accessible in GDScript using `my_struct.my_field` syntax. Additionally, it
/// generates a trivial getter and setter named `get_my_field` and `set_my_field`, respectively.
/// These are `pub` in Rust, since they're exposed from GDScript anyway.
///
/// For technical reasons, an impl-block with the `#[godot_api]` attribute is required for properties to
/// work. Failing to include one will cause a compile error if you try to create any properties.
///
/// If you want to implement your own getter and/or setter, write those as a function on your Rust
/// type, expose it using `#[func]`, and annotate the field with
/// `#[export(get = ..., set = ...)]`:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     #[var(get = get_my_field, set = set_my_field)]
///     my_field: i64,
/// }
///
/// #[godot_api]
/// impl MyStruct {
///     #[func]
///     pub fn get_my_field(&self) -> i64 {
///         self.my_field
///     }
///
///     #[func]
///     pub fn set_my_field(&mut self, value: i64) {
///         self.my_field = value;
///     }
/// }
/// ```
///
/// If you specify only `get`, no setter is generated, making the field read-only. If you specify
/// only `set`, no getter is generated, making the field write-only (rarely useful). To add a
/// generated getter or setter in these cases anyway, use `get` or `set` without a value:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     // Default getter, custom setter.
///     #[var(get, set = set_my_field)]
///     my_field: i64,
/// }
///
/// #[godot_api]
/// impl MyStruct {
///     #[func]
///     pub fn set_my_field(&mut self, value: i64) {
///         self.my_field = value;
///     }
/// }
/// ```
///
/// For exporting properties to the editor, you can use the `#[export]` attribute:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     #[export]
///     my_field: i64,
/// }
///
/// #[godot_api]
/// impl MyStruct {}
/// ```
///
/// If you dont also include a `#[var]` attribute, then a default one will be generated.
/// `#[export]` also supports all of GDScript's annotations, in a slightly different format. The format is
/// translated from an annotation by following these four rules:
///
/// - `@export` becomes `#[export]`
/// - `@export_{name}` becomes `#[export(name)]`
/// - `@export_{name}(elem1, ...)` becomes `#[export(name = (elem1, ...))]`
/// - `@export_{flags/enum}("elem1", "elem2:key2", ...)`
///   becomes
///   `#[export(flags/enum = (elem1, elem2 = key2, ...))]`
///
///
/// As an example of some different export attributes:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     // @export
///     #[export]
///     float: f64,
///     
///     // @export_range(0.0, 10.0, or_greater)
///     #[export(range = (0.0, 10.0, or_greater))]
///     range_f64: f64,
///
///     // @export_file
///     #[export(file)]
///     file: GodotString,
///
///     // @export_file("*.gd")
///     #[export(file = "*.gd")]
///     gdscript_file: GodotString,
///
///     // @export_flags_3d_physics
///     #[export(flags_3d_physics)]
///     physics: u32,
///
///     // @export_exp_easing
///     #[export(exp_easing)]
///     ease: f64,
///
///     // @export_enum("One", "Two", "Ten:10", "Twelve:12", "Thirteen")
///     #[export(enum = (One, Two, Ten = 10, Twelve = 12, Thirteen))]
///     exported_enum: i64,
///
///     // @export_flags("A:1", "B:2", "AB:3")
///     #[export(flags = (A = 1, B = 2, AB = 3))]
///     flags: u32,
/// }
///
/// #[godot_api]
/// impl MyStruct {}
/// ```
///
/// Most values in expressions like `key = value`, can be an arbitrary expression that evaluates to the
/// right value. Meaning you can use constants or variables, as well as any other rust syntax you'd like in
/// the export attributes.
///
/// ```
/// use godot::prelude::*;
///
/// const MAX_HEALTH: f64 = 100.0;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     #[export(range = (0.0, MAX_HEALTH))]
///     health: f64,
///
///     #[export(flags = (A = 0b0001, B = 0b0010, C = 0b0100, D = 0b1000))]
///     flags: u32,
/// }
///
/// #[godot_api]
/// impl MyStruct {}
/// ```
///
/// You can specify custom property hints, hint strings, and usage flags in a `#[var]` attribute using the
/// `hint`, `hint_string`, and `usage_flags` keys in the attribute:
///
/// ```
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct {
///     // Treated as an enum with two values: "One" and "Two"
///     // Displayed in the editor
///     // Treated as read-only by the editor
///     #[var(
///         hint = PROPERTY_HINT_ENUM,
///         hint_string = "One,Two",
///         usage_flags = [PROPERTY_USAGE_EDITOR, PROPERTY_USAGE_READ_ONLY]
///     )]
///     my_field: i64,
/// }
///
/// #[godot_api]
/// impl MyStruct {}
/// ```
///
///
/// # Signals
///
/// The `#[signal]` attribute is accepted, but not yet implemented. See [issue
/// #8](https://github.com/godot-rust/gdext/issues/8).
#[proc_macro_derive(GodotClass, attributes(class, base, var, export, init, signal))]
pub fn derive_native_class(input: TokenStream) -> TokenStream {
    translate(input, derive_godot_class::transform)
}

/// Derive macro for [ToVariant](godot::builtin::ToVariant) on structs or enums.
///
/// Example :
///
/// ```ignore
/// #[derive(FromVariant, ToVariant, PartialEq, Debug)]
/// struct StructNamed {
///     field1: String,
///     field2: i32,
/// }
///
/// // This would not panic.
/// assert!(
///     StructNamed {
///         field1: "1".to_string(),
///         field2: 2,
///     }
///     .to_variant()
///         == dict! {
///           "StructNamed":dict!{
///             "field1":"four","field2":5
///           }
///         }
///         .to_variant()
/// );
/// ```
///
/// You can use the skip attribute to ignore a field from being converted to ToVariant.
#[proc_macro_derive(ToVariant, attributes(variant))]
pub fn derive_to_variant(input: TokenStream) -> TokenStream {
    translate(input, derive_to_variant::transform)
}

/// Derive macro for [FromVariant](godot::builtin::FromVariant) on structs or enums.
///
/// Example :
///
/// ```ignore
/// #[derive(FromVariant, ToVariant, PartialEq, Debug)]
/// struct StructNamed {
///     field1: String,
///     field2: i32,
/// }
///
/// // This would not panic.
/// assert!(
///     StructNamed::from_variant(
///         &dict! {
///           "StructNamed":dict!{
///             "field1":"four","field2":5
///           }
///         }
///         .to_variant()
///     ) == StructNamed {
///         field1: "1".to_string(),
///         field2: 2,
///     }
/// );
/// ```
///
/// You can use the skip attribute to ignore a field from the provided variant and use `Default::default()`
/// to get it instead.
#[proc_macro_derive(FromVariant, attributes(variant))]
pub fn derive_from_variant(input: TokenStream) -> TokenStream {
    translate(input, derive_from_variant::transform)
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
