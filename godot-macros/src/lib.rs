/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Internal crate of [**godot-rust**](https://godot-rust.github.io)
//!
//! Do not depend on this crate directly, instead use the `godot` crate.
//! No SemVer or other guarantees are provided.

mod bench;
mod class;
mod derive;
mod gdextension;
mod itest;
mod util;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::util::ident;

// Below intra-doc link to the trait only works as HTML, not as symbol link.
/// Derive macro for [`GodotClass`](../obj/trait.GodotClass.html) on structs.
///
/// You should use this macro; manual implementations of the `GodotClass` trait are not encouraged.
///
/// This is typically used in combination with [`#[godot_api]`](attr.godot_api.html), which can implement custom functions and constants,
/// as well as override virtual methods.
///
/// See also [book chapter _Registering classes_](https://godot-rust.github.io/book/register/classes.html).
///
/// **Table of contents:**
/// - [Construction](#construction)
/// - [Inheritance](#inheritance)
/// - [Properties and exports](#properties-and-exports)
///    - [Property registration](#property-registration)
///    - [Property exports](#property-exports)
/// - [Signals](#signals)
/// - [Further class customization](#further-class-customization)
///    - [Running code in the editor](#running-code-in-the-editor)
///    - [Editor plugins](#editor-plugins)
///    - [Class renaming](#class-renaming)
///    - [Class hiding](#class-hiding)
/// - [Further field customization](#further-field-customization)
///    - [Fine-grained inference hints](#fine-grained-inference-hints)
///
///
/// # Construction
///
/// If you don't override `init()` manually (within a `#[godot_api]` block), gdext can generate a default constructor for you.
/// This constructor is made available to Godot and lets you call `MyStruct.new()` from GDScript. To enable it, annotate your
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
/// The generated `init` function will initialize each struct field (except the field of type `Base<T>`, if any)
/// using `Default::default()`. To assign some other value, annotate the field with `#[init(default = ...)]`:
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
/// #[init(default = (HashMap::<i64, i64>::new()))]
/// //                             ^ parentheses needed due to this comma
/// my_field: HashMap<i64, i64>,
/// # }
/// ```
///
/// You can also _disable_ construction from GDScript. This needs to be explicit via `#[class(no_init)]`.
/// Simply omitting the `init`/`no_init` keys and not overriding your own constructor will cause a compile error.
///
/// ```
/// # use godot_macros::GodotClass;
/// #[derive(GodotClass)]
/// #[class(no_init)]
/// struct MyStruct {
///    // ...
/// }
/// ```
///
/// # Inheritance
///
/// Unlike C++, Rust doesn't really have inheritance, but the GDExtension API lets us "inherit"
/// from a Godot-provided engine class.
///
/// By default, classes created with this library inherit from `RefCounted`, like GDScript.
///
/// To specify a different class to inherit from, add `#[class(base = Base)]` as an annotation on
/// your `struct`:
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node2D)]
/// struct MyStruct {
///     // ...
/// }
/// ```
///
/// If you need a reference to the base class, you can add a field of type `Base<T>`. The derive macro will pick this up and wire
/// your object accordingly. You can access it through `self.base()` and `self.base_mut()` methods.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node2D)]
/// struct MyStruct {
///     base: Base<Node2D>,
/// }
/// ```
///
///
/// # Properties and exports
///
/// See also [book chapter _Registering properties_](https://godot-rust.github.io/book/register/properties.html#registering-properties).
///
/// In GDScript, there is a distinction between
/// [properties](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html#properties-setters-and-getters)
/// (fields with a `get` or `set` declaration) and
/// [exports](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_exports.html)
/// (fields annotated with `@export`). In the gdext API, these two concepts are represented with `#[var]` and `#[export]` attributes respectively.
///
/// ## Property registration
///
/// To create a property, you can use the `#[var]` annotation:
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
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
/// `#[var(get = ..., set = ...)]`:
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
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
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
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
/// ## Property exports
///
/// For exporting properties to the editor, you can use the `#[export]` attribute:
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct MyStruct {
///     #[export]
///     my_field: i64,
/// }
/// ```
///
/// If you don't also include a `#[var]` attribute, then a default one will be generated.
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
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
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
///
/// ```
///
/// Most values in expressions like `key = value`, can be an arbitrary expression that evaluates to the
/// right value. Meaning you can use constants or variables, as well as any other rust syntax you'd like in
/// the export attributes.
///
/// ```
/// # use godot::prelude::*;
/// const MAX_HEALTH: f64 = 100.0;
///
/// #[derive(GodotClass)]
/// # #[class(init)]
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
/// `hint`, `hint_string`, and `usage_flags` keys in the attribute. These are constants in the `PropertyHint`
/// and `PropertyUsageFlags` enums, respectively.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct MyStruct {
///     // Treated as an enum with two values: "One" and "Two"
///     // Displayed in the editor
///     // Treated as read-only by the editor
///     #[var(
///         hint = ENUM,
///         hint_string = "One,Two",
///         usage_flags = [EDITOR, READ_ONLY]
///     )]
///     my_field: i64,
/// }
/// ```
///
/// # Signals
///
/// The `#[signal]` attribute is quite limited at the moment. The functions it decorates (the signals) can accept parameters.
/// It will be fundamentally reworked.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct MyClass {}
///
/// #[godot_api]
/// impl MyClass {
///     #[signal]
///     fn some_signal();
///
///     #[signal]
///     fn some_signal_with_parameters(my_parameter: Gd<Node>);
/// }
/// ```
///
/// # Further class customization
///
/// ## Running code in the editor
///
/// If you annotate a class with `#[class(tool)]`, its lifecycle methods (`ready()`, `process()` etc.) will be invoked in the editor. This
/// is useful for writing custom editor plugins, as opposed to classes running simply in-game.
///
/// See [`ExtensionLibrary::editor_run_behavior()`](../init/trait.ExtensionLibrary.html#method.editor_run_behavior)
/// for more information and further customization.
///
/// This is very similar to [GDScript's `@tool` feature](https://docs.godotengine.org/en/stable/tutorials/plugins/running_code_in_the_editor.html).
///
/// ## Editor plugins
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
/// ## Class renaming
///
/// You may want to have structs with the same name. With Rust, this is allowed using `mod`. However in GDScript,
/// there are no modules, namespaces, or any such disambiguation.  Therefore, you need to change the names before they
/// can get to Godot. You can use the `rename` key while defining your `GodotClass` for this.
///
/// ```no_run
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
///
/// ## Class hiding
///
/// If you want to register a class with Godot, but not have it show up in the editor then you can use `#[class(hidden)]`.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(base=Node, init, hidden)]
/// pub struct Foo {}
/// ```
///
/// Even though this class is a `Node` and it has an init function, it still won't show up in the editor as a node you can add to a scene
/// because we have added a `hidden` key to the class. This will also prevent it from showing up in documentation.
///
/// # Further field customization
///
/// ## Fine-grained inference hints
///
/// The derive macro is relatively smart about recognizing `Base<T>` and `OnReady<T>` types, and works also if those are qualified.
///
/// However, there may be situations where you need to help it out -- for example, if you have a type alias for `Base<T>`, or use an unrelated
/// `my_module::Base<T>` with a different meaning.
///
/// In this case, you can manually override the behavior with the `#[hint]` attribute. It takes multiple standalone keys:
/// - `base` and `no_base`
/// - `onready` and `no_onready`
///
/// ```no_run
/// use godot::classes::Node;
///
/// // There's no reason to do this, but for the sake of example:
/// type Super<T> = godot::obj::Base<T>;
/// type Base<T> = godot::obj::Gd<T>;
///
/// #[derive(godot::register::GodotClass)]
/// #[class(base=Node)]
/// struct MyStruct {
///    #[hint(base)]
///    base: Super<Node>,
///
///    #[hint(no_base)]
///    unbase: Base<Node>,
/// }
/// # #[godot::register::godot_api]
/// # impl godot::classes::INode for MyStruct {
/// #     fn init(base: godot::obj::Base<Self::Base>) -> Self { todo!() }
/// # }
/// ```
#[proc_macro_derive(GodotClass, attributes(class, base, hint, var, export, init, signal))]
pub fn derive_godot_class(input: TokenStream) -> TokenStream {
    translate(input, class::derive_godot_class)
}

/// Proc-macro attribute to be used with `impl` blocks of [`#[derive(GodotClass)]`][GodotClass] structs.
///
/// Can be used in two ways:
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// struct MyClass {}
///
/// // 1) inherent impl block: user-defined, custom API.
/// #[godot_api]
/// impl MyClass { /* ... */ }
///
/// // 2) trait impl block: implement Godot-specific APIs.
/// #[godot_api]
/// impl INode for MyClass { /* ... */ }
/// ```
///
/// The second case works by implementing the corresponding trait `I*` for the base class of your class
/// (for example `IRefCounted` or `INode3D`). Then, you can add functionality such as:
/// * `init` constructors
/// * lifecycle methods like `ready` or `process`
/// * `on_notification` method
/// * `to_string` method
///
/// Neither of the two `#[godot_api]` blocks is required. For small data bundles inheriting `RefCounted`, you may be fine with
/// accessing properties directly from GDScript.
///
/// See also [book chapter _Registering functions_](https://godot-rust.github.io/book/register/functions.html) and following.
///
/// **Table of contents**
/// - [Constructors](#constructors)
///   - [User-defined `init`](#user-defined-init)
///   - [Generated `init`](#generated-init)
/// - [Lifecycle functions](#lifecycle-functions)
/// - [User-defined functions](#user-defined-functions)
///   - [Associated functions and methods](#associated-functions-and-methods)
///   - [Virtual methods](#virtual-methods)
/// - [Constants and signals](#signals)
///
/// # Constructors
///
/// Note that `init` (the Godot default constructor) can be either provided by overriding it, or generated with a `#[class(init)]` attribute
/// on the struct. Classes without `init` cannot be instantiated from GDScript.
///
/// ## User-defined `init`
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// // no #[class(init)] here, since init() is overridden below.
/// // #[class(base=RefCounted)] is implied if no base is specified.
/// struct MyStruct;
///
/// #[godot_api]
/// impl IRefCounted for MyStruct {
///     fn init(_base: Base<RefCounted>) -> Self {
///         MyStruct
///     }
/// }
/// ```
///
/// ## Generated `init`
///
/// This initializes the `Base<T>` field, and every other field with either `Default::default()` or the value specified in `#[init(default = ...)]`.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode {
///     base: Base<Node>,
///
///     #[init(default = 42)]
///     some_integer: i64,
/// }
/// ```
///
///
/// # Lifecycle functions
///
/// You can override the lifecycle functions `ready`, `process`, `physics_process` and so on, by implementing the trait corresponding to the
/// base class.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode;
///
/// #[godot_api]
/// impl INode for MyNode {
///     fn ready(&mut self) {
///         godot_print!("Hello World!");
///     }
/// }
/// ```
///
///
/// # User-defined functions
///
/// You can use the `#[func]` attribute to declare your own functions. These are exposed to Godot and callable from GDScript.
///
/// ## Associated functions and methods
///
/// If `#[func]` functions are called from the engine, they implicitly bind the surrounding `Gd<T>` pointer: `Gd::bind()` in case of `&self`,
/// `Gd::bind_mut()` in case of `&mut self`. To avoid that, use `#[func(gd_self)]`, which requires an explicit first argument of type `Gd<T>`.
///
/// Functions without a receiver become static functions in Godot. They can be called from GDScript using `MyStruct.static_function()`.
/// If they return `Gd<Self>`, they are effectively constructors that allow taking arguments.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyStruct {
///     field: i64,
///     base: Base<RefCounted>,
/// }
///
/// #[godot_api]
/// impl MyStruct {
///     #[func]
///     pub fn hello_world(&mut self) {
///         godot_print!("Hello World!")
///     }
///
///     #[func]
///     pub fn static_function(constructor_arg: i64) -> Gd<Self> {
///         Gd::from_init_fn(|base| {
///            MyStruct { field: constructor_arg, base }
///         })
///     }
///
///     #[func(gd_self)]
///     pub fn explicit_receiver(mut this: Gd<Self>, other_arg: bool) {
///         // Only bind Gd pointer if needed.
///         if other_arg {
///             this.bind_mut().field = 55;
///         }
///     }
/// }
/// ```
///
/// ## Virtual methods
///
/// Functions with the `#[func(virtual)]` attribute are virtual functions, meaning attached scripts can override them.
///
/// ```no_run
/// # #[cfg(since_api = "4.3")]
/// # mod conditional {
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyStruct {
///     // Virtual functions require base object.
///     base: Base<RefCounted>,
/// }
///
/// #[godot_api]
/// impl MyStruct {
///     #[func(virtual)]
///     fn language(&self) -> GString {
///         "Rust".into()
///     }
/// }
/// # }
/// ```
///
/// In GDScript, your method is available with a `_` prefix, following Godot convention for virtual methods:
/// ```gdscript
/// extends MyStruct
///
/// func _language():
///    return "GDScript"
/// ```
///
/// Now, `obj.language()` from Rust will dynamically dispatch the call.
///
/// Make sure you understand the limitations in the [tutorial](https://godot-rust.github.io/book/register/virtual-functions.html).
///
/// # Constants and signals
///
/// Please refer to [the book](https://godot-rust.github.io/book/register/constants.html).
#[proc_macro_attribute]
pub fn godot_api(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, class::attribute_godot_api)
}

/// Derive macro for [`GodotConvert`](../builtin/meta/trait.GodotConvert.html) on structs.
///
/// This derive macro also derives [`ToGodot`](../builtin/meta/trait.ToGodot.html) and [`FromGodot`](../builtin/meta/trait.FromGodot.html).
///
/// # Choosing a Via type
///
/// To specify the `Via` type that your type should be converted to, you must use the `godot` attribute.
/// There are currently two modes supported.
///
/// ## `transparent`
///
/// If you specify `#[godot(transparent)]` on single-field struct, your struct will be treated as a newtype struct. This means that all derived
/// operations on the struct will defer to the type of that single field.
///
/// ### Example
///
/// ```no_run
/// use godot::prelude::*;
///
/// #[derive(GodotConvert)]
/// #[godot(transparent)]
/// struct CustomVector2(Vector2);
///
/// let obj = CustomVector2(Vector2::new(10.0, 25.0));
/// assert_eq!(obj.to_godot(), Vector2::new(10.0, 25.0));
/// ```
///
/// This also works for named structs with a single field:
/// ```no_run
/// use godot::prelude::*;
///
/// #[derive(GodotConvert)]
/// #[godot(transparent)]
/// struct MyNewtype {
///     string: GString,
/// }
///
/// let obj = MyNewtype {
///     string: "hello!".into(),
/// };
/// assert_eq!(obj.to_godot(), GString::from("hello!"));
/// ```
///
/// However it will not work for structs with more than one field, even if that field is zero sized:
/// ```compile_fail
/// use godot::prelude::*;
///
/// #[derive(GodotConvert)]
/// #[godot(transparent)]
/// struct SomeNewtype {
///     int: i64,
///     zst: (),
/// }
/// ```
///
/// You can also not use `transparent` with enums:
/// ```compile_fail
/// use godot::prelude::*;
///
/// #[derive(GodotConvert)]
/// #[godot(transparent)]
/// enum MyEnum {
///     Int(i64)
/// }
/// ```
///
/// ## `via = <type>`
///
/// For c-style enums, that is enums where all the variants are unit-like, you can use `via = <type>` to convert the enum into that
/// type.
///
/// The types you can use this with currently are:
/// - `GString`
/// - `i8`, `i16`, `i32`, `i64`
/// - `u8`, `u16`, `u32`
///
/// When using one of the integer types, each variant of the enum will be converted into its discriminant.
///
/// ### Examples
///
/// ```no_run
/// use godot::prelude::*;
/// #[derive(GodotConvert)]
/// #[godot(via = GString)]
/// enum MyEnum {
///     A,
///     B,
///     C,
/// }
///
/// assert_eq!(MyEnum::A.to_godot(), GString::from("A"));
/// assert_eq!(MyEnum::B.to_godot(), GString::from("B"));
/// assert_eq!(MyEnum::C.to_godot(), GString::from("C"));
/// ```
///
/// ```no_run
/// use godot::prelude::*;
/// #[derive(GodotConvert)]
/// #[godot(via = i64)]
/// enum MyEnum {
///     A,
///     B,
///     C,
/// }
///
/// assert_eq!(MyEnum::A.to_godot(), 0);
/// assert_eq!(MyEnum::B.to_godot(), 1);
/// assert_eq!(MyEnum::C.to_godot(), 2);
/// ```
///
/// Explicit discriminants are used for integers:
///
/// ```no_run
/// use godot::prelude::*;
/// #[derive(GodotConvert)]
/// #[godot(via = u8)]
/// enum MyEnum {
///     A,
///     B = 10,
///     C,
/// }
///
/// assert_eq!(MyEnum::A.to_godot(), 0);
/// assert_eq!(MyEnum::B.to_godot(), 10);
/// assert_eq!(MyEnum::C.to_godot(), 11);
/// ```
#[proc_macro_derive(GodotConvert, attributes(godot))]
pub fn derive_godot_convert(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_godot_convert)
}

/// Derive macro for [`Var`](../register/property/trait.Var.html) on enums.
///
/// This expects a derived [`GodotConvert`](../builtin/meta/trait.GodotConvert.html) implementation, using a manual
/// implementation of `GodotConvert` may lead to incorrect values being displayed in Godot.
#[proc_macro_derive(Var, attributes(godot))]
pub fn derive_var(input: TokenStream) -> TokenStream {
    translate(input, derive::derive_var)
}

/// Derive macro for [`Export`](../register/property/trait.Export.html) on enums.
///
/// See also [`Var`].
#[proc_macro_derive(Export, attributes(godot))]
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
/// [`ExtensionLibrary`]: ../init/trait.ExtensionLibrary.html
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
    F: FnOnce(venial::Item) -> ParseResult<TokenStream2>,
{
    let input2 = TokenStream2::from(input);

    let result2 = venial::parse_item(input2)
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
    F: FnOnce(venial::Item) -> ParseResult<TokenStream2>,
{
    let self_name = ident(self_name);
    let input2 = TokenStream2::from(input);
    let meta2 = TokenStream2::from(meta);

    // Hack because venial doesn't support direct meta parsing yet
    let input = quote! {
        #[#self_name(#meta2)]
        #input2
    };

    let result2 = venial::parse_item(input)
        .and_then(transform)
        .unwrap_or_else(|e| e.to_compile_error());

    TokenStream::from(result2)
}
