/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod bench;
mod class;
mod derive;
mod gdextension;
mod itest;
mod util;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use venial::Declaration;

use crate::util::ident;

// Below intra-doc link to the trait only works as HTML, not as symbol link.
/// Derive macro for [the `GodotClass` trait](../obj/trait.GodotClass.html) on structs.
///
/// You must use this macro; manual implementations of the `GodotClass` trait are not supported.
///
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
///     base: Base<Node2D>,
/// }
/// ```
///
///
/// # Properties and exports
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
/// ```
///
/// This makes the field accessible in GDScript using `my_struct.my_field` syntax. Additionally, it
/// generates a trivial getter and setter named `get_my_field` and `set_my_field`, respectively.
/// These are `pub` in Rust, since they're exposed from GDScript anyway.
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
///     file: GString,
///
///     // @export_file("*.gd")
///     #[export(file = "*.gd")]
///     gdscript_file: GString,
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
/// ```
///
///
/// # Signals
///
/// The `#[signal]` attribute is accepted, but not yet implemented. See [issue
/// #8](https://github.com/godot-rust/gdext/issues/8).
///
///
/// # Running code in the editor
///
/// If you annotate a class with `#[class(tool)]`, its lifecycle methods (`ready()`, `process()` etc.) will be invoked in the editor. This
/// is useful for writing custom editor plugins, as opposed to classes running simply in-game.
///
/// See [`ExtensionLibrary::editor_run_behavior()`](../init/trait.ExtensionLibrary.html#method.editor_run_behavior)
/// for more information and further customization.
///
/// This is very similar to [GDScript's `@tool` feature](https://docs.godotengine.org/en/stable/tutorials/plugins/running_code_in_the_editor.html).
///
/// # Editor Plugins
///
/// If you annotate a class with `#[class(editor_plugin)]`, it will be turned into an editor plugin. The
/// class must then inherit from `EditorPlugin`, and an instance of that class will be automatically added
/// to the editor when launched.
///
/// See [Godot's documentation of editor plugins](https://docs.godotengine.org/en/stable/tutorials/plugins/editor/index.html)
/// for more information about editor plugins. But note that you do not need to create and enable the plugin
/// through Godot's `Create New Plugin` menu for it to work, simply annotating the class with `editor_plugin`
/// automatically enables it when the library is loaded.
///
/// This should usually be combined with `#[class(tool)]` so that the code you write will actually run in the
/// editor.
///
/// # Class Renaming
///
/// You may want to have structs with the same name. With Rust, this is allowed using `mod`. However in GDScript,
/// there are no modules, namespaces, or any such disambiguation.  Therefore, you need to change the names before they
/// can get to Godot. You can use the `rename` key while defining your `GodotClass` for this.
///
/// ```
/// mod animal {
///     # use godot::prelude::*;
///     #[derive(GodotClass)]
///     #[class(init, rename=AnimalToad)]
///     pub struct Toad {}
/// }
///
/// mod npc {
///     # use godot::prelude::*;
///     #[derive(GodotClass)]
///     #[class(init, rename=NpcToad)]
///     pub struct Toad {}
/// }
/// ```
///
/// These classes will appear in the Godot editor and GDScript as "AnimalToad" or "NpcToad".
#[proc_macro_derive(GodotClass, attributes(class, base, var, export, init, signal))]
pub fn derive_godot_class(input: TokenStream) -> TokenStream {
    translate(input, class::derive_godot_class)
}

/// Proc-macro attribute to be used with `impl` blocks of `#[derive(GodotClass)]` structs.
///
/// Can be used in two ways:
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(base=Node)]
/// struct MyClass {}
///
/// // 1) inherent impl block: user-defined, custom API
/// #[godot_api]
/// impl MyClass { /* ... */ }
///
/// // 2) trait impl block: implement Godot-specific APIs
/// #[godot_api]
/// impl INode for MyClass { /* ... */ }
/// ```
///
/// The second case works by implementing the corresponding trait `I<Base>` for the base class of your class
/// (for example `IRefCounted` or `INode3D`). Then, you can add functionality such as:
/// * `init` constructors
/// * lifecycle methods like `ready` or `process`
/// * `on_notification` method
/// * `to_string` method
///
/// Neither `#[godot_api]` attribute is required. For small data bundles inheriting `RefCounted`, you may be fine with
/// accessing properties directly from GDScript.
///
/// # Examples
///
/// ## `RefCounted` as a base, overridden `init`
///
/// ```no_run
///# use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// struct MyStruct;
///
/// #[godot_api]
/// impl MyStruct {
///     #[func]
///     pub fn hello_world(&mut self) {
///         godot_print!("Hello World!")
///     }
/// }
///
/// #[godot_api]
/// impl IRefCounted for MyStruct {
///     fn init(_base: Base<RefCounted>) -> Self {
///         MyStruct
///     }
/// }
/// ```
///
/// Note that `init` can be either provided by overriding it, or generated with a `#[class(init)]` attribute on the struct.
/// Classes without `init` cannot be instantiated from GDScript.
///
/// ## `Node` as a base, generated `init`
///
/// ```no_run
///# use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode {
///     #[base]
///     base: Base<Node>,
/// }
///
/// #[godot_api]
/// impl INode for MyNode {
///     fn ready(&mut self) {
///         godot_print!("Hello World!");
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn godot_api(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, class::attribute_godot_api)
}

#[proc_macro_derive(GodotConvert)]
pub fn derive_godot_convert(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_godot_convert)
}

/// Derive macro for [ToGodot](../builtin/meta/trait.ToGodot.html) on structs or enums.
///
/// # Example
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
/// struct StructNamed {
///     field1: String,
///     field2: i32,
/// }
///
/// let obj = StructNamed {
///     field1: "1".to_string(),
///     field2: 2,
/// };
/// let dict = dict! {
///    "StructNamed": dict! {
///        "field1": "four",
///        "field2": 5,
///    }
/// };
///
/// // This would not panic.
/// assert_eq!(obj.to_variant(), dict.to_variant());
/// ```
///
/// You can use the `#[skip]` attribute to ignore a field from being converted to `ToGodot`.
#[proc_macro_derive(ToGodot, attributes(variant))]
pub fn derive_to_godot(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_to_godot)
}

/// Derive macro for [FromGodot](../builtin/meta/trait.FromVariant.html) on structs or enums.
///
/// # Example
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
/// struct StructNamed {
///     field1: String,
///     field2: i32,
/// }
///
/// let obj = StructNamed {
///     field1: "1".to_string(),
///     field2: 2,
/// };
/// let dict_variant = dict! {
///    "StructNamed": dict! {
///        "field1": "four",
///        "field2": 5,
///    }
/// }.to_variant();
///
/// // This would not panic.
/// assert_eq!(StructNamed::from_variant(&dict_variant), obj);
/// ```
///
/// You can use the skip attribute to ignore a field from the provided variant and use `Default::default()`
/// to get it instead.
#[proc_macro_derive(FromGodot, attributes(variant))]
pub fn derive_from_godot(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_from_godot)
}

/// Derive macro for [Property](../bind/property/trait.Property.html) on enums.
///
/// Currently has some tight requirements which are expected to be softened as implementation expands:
/// - Only works for enums, structs aren't supported by this derive macro at the moment.
/// - The enum must have an explicit `#[repr(u*/i*)]` type.
///     - This will likely stay this way, since `isize`, the default repr type, is not a concept in Godot.
/// - The enum variants must not have any fields - currently only unit variants are supported.
/// - The enum variants must have explicit discriminants, that is, e.g. `A = 2`, not just `A`
///
/// # Example
///
/// ```no_run
/// # use godot::prelude::*;
/// #[repr(i32)]
/// #[derive(Property)]
/// # #[derive(Eq, PartialEq, Debug)]
/// enum TestEnum {
///     A = 0,
///     B = 1,
/// }
///
/// #[derive(GodotClass)]
/// struct TestClass {
///     #[var]
///     foo: TestEnum
/// }
///
/// # fn main() {
/// let mut class = TestClass {foo: TestEnum::B};
/// assert_eq!(class.get_foo(), TestEnum::B as i32);
/// class.set_foo(TestEnum::A as i32);
/// assert_eq!(class.foo, TestEnum::A);
/// # }
/// ```
#[proc_macro_derive(Property)]
pub fn derive_property(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_property)
}

/// Derive macro for [Export](../bind/property/trait.Export.html) on enums.
///
/// Currently has some tight requirements which are expected to be softened as implementation expands, see requirements for [Property].
#[proc_macro_derive(Export)]
pub fn derive_export(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_export)
}

/// Similar to `#[test]`, but runs an integration test with Godot.
///
/// Transforms the `fn` into one returning `bool` (success of the test), which must be called explicitly.
#[proc_macro_attribute]
pub fn itest(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate_meta("itest", meta, input, itest::attribute_itest)
}

/// Similar to `#[test]`, but runs an benchmark with Godot.
///
/// Calls the `fn` many times and gathers statistics from its execution time.
#[proc_macro_attribute]
pub fn bench(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate_meta("bench", meta, input, bench::attribute_bench)
}

/// Proc-macro attribute to be used in combination with the [`ExtensionLibrary`] trait.
///
/// [`ExtensionLibrary`]: trait.ExtensionLibrary.html
// FIXME intra-doc link
#[proc_macro_attribute]
pub fn gdextension(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate_meta(
        "gdextension",
        meta,
        input,
        gdextension::attribute_gdextension,
    )
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
