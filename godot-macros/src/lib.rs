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
#[cfg(all(feature = "register-docs", since_api = "4.3"))]
mod docs;
mod ffi_macros;
mod gdextension;
mod itest;
mod util;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use crate::util::{bail, ident, KvParser};

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
/// **See sidebar on the left for table of contents.**
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
/// using `Default::default()`. To assign some other value, annotate the field with `#[init(val = ...)]`:
///
/// ```
/// # use godot_macros::GodotClass;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyStruct {
///     #[init(val = 42)]
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
/// #[init(val = (HashMap::<i64, i64>::new()))]
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
/// (fields annotated with `@export`). In the gdext API, these two concepts are represented with `#[var]` and `#[export]` attributes respectively,
/// which in turn are backed by the [`Var`](../register/property/trait.Var.html) and [`Export`](../register/property/trait.Export.html) traits.
///
/// ## Register properties -- `#[var]`
///
/// To create a property, you can use the `#[var]` annotation, which supports types implementing [`Var`](../register/property/trait.Var.html).
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
/// #[class(init)]
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
/// To create a property without a backing field to store data, you can use [`PhantomVar`](../obj/struct.PhantomVar.html).
/// This disables autogenerated getters and setters for that field.
///
/// ## Export properties -- `#[export]`
///
/// To export properties to the editor, you can use the `#[export]` attribute, which supports types implementing
/// [`Export`](../register/property/trait.Export.html):
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
/// If you don't include an additional `#[var]` attribute, then a default one will be generated.
///
/// `#[export]` also supports all of [GDScript's annotations][gdscript-annotations], in a slightly different format. The format is
/// translated from an annotation by following these four rules:
///
/// [gdscript-annotations]: https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_exports.html
///
/// | GDScript annotation                         | Rust attribute                                 |
/// |---------------------------------------------|-------------------------------------------------|
/// | `@export`                                   | `#[export]`                                     |
/// | `@export_key`                               | `#[export(key)]`                                |
/// | `@export_key(elem1, ...)`                   | `#[export(key = (elem1, ...))]`                 |
/// | `@export_flags("elem1", "elem2:val2", ...)`<br>`@export_enum("elem1", "elem2:val2", ...)` | `#[export(flags = (elem1, elem2 = val2, ...))]`<br>`#[export(enum = (elem1, elem2 = val2, ...))]` |
///
/// As an example of different export attributes:
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
///     // @export_storage
///     #[export(storage)]
///     hidden_string: GString,
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
/// Most values in syntax such as `key = value` can be arbitrary expressions. For example, you can use constants, function calls or
/// other Rust expressions that are valid in that context.
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
/// It is possible to group your exported properties inside the Inspector with the `#[export_group(name = "...", prefix =  "...")]` attribute.
/// Every exported property after this attribute will be added to the group. Start a new group or use `#[export_group(name = "")]` (with an empty name) to break out.
///
/// Groups cannot be nested but subgroups can be declared with an `#[export_subgroup]` attribute.
///
/// GDExtension groups and subgroups follow the same rules as the gdscript ones.
///
/// <div class="warning">
/// Nesting subgroups with the slash separator `/` <strong>outside</strong> the group is not supported and might crash the editor.
/// </div>
///
/// See also in Godot docs:
/// [Grouping Exports](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_exports.html#grouping-exports)
///
///```
/// # use godot::prelude::*;
/// const MAX_HEALTH: f64 = 100.0;
///
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct MyStruct {
///     // @export_group("Group 1")
///     // @export var group_1_field: int
///     #[export]
///     #[export_group(name = "Group 1")]
///     group_1_field: i32,
///
///     // @export var group_1_field2: int
///     #[export]
///     group_1_field2: i32,
///
///     // @export_group("my group", "grouped_")
///     // @export var grouped_field: int
///     #[export_group(name = "my group", prefix = "grouped_")]
///     #[export]
///     grouped_field: u32,
///
///     // @export_subgroup("my subgroup")
///     // @export var sub_field: int
///     #[export_subgroup(name = "my subgroup")]
///     #[export]
///     sub_field: u32,
///
///     // Breaks out of subgroup `"my subgroup"`.
///     // @export_subgroup("")
///     // @export var grouped_field2: int
///     #[export_subgroup(name = "")]
///     #[export]
///     grouped_field2: u32,
///
///     // @export var ungrouped_field: int
///     #[export]
///     ungrouped_field: i64,
/// }
///```
///
///
/// ## Low-level property hints and usage
///
/// You can specify custom property hints, hint strings, and usage flags in a `#[var]` attribute using the `hint`, `hint_string`
/// and `usage_flags` keys in the attribute. Hint and usage flags are constants in the [`PropertyHint`] and [`PropertyUsageFlags`] enums,
/// while hint strings are dependent on the hint, property type and context. Using these low-level keys is rarely necessary, as most common
/// combinations are covered by `#[var]` and `#[export]` already.
///
/// [`PropertyHint`]: ../global/struct.PropertyHint.html
/// [`PropertyUsageFlags`]: ../global/struct.PropertyUsageFlags.html
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct MyStruct {
///     // Treated as an enum with two values: "One" and "Two",
///     // displayed in the editor,
///     // treated as read-only by the editor.
///     #[var(
///         hint = ENUM,
///         hint_string = "One,Two",
///         usage_flags = [EDITOR, READ_ONLY]
///     )]
///     my_field: i64,
/// }
/// ```
///
///
/// # Further class customization
///
/// ## Running code in the editor (tool)
///
/// If you annotate a class with `#[class(tool)]`, its lifecycle methods (`ready()`, `process()` etc.) will be invoked in the editor. This
/// is useful for writing custom editor plugins, as opposed to classes running simply in-game.
///
/// See [`ExtensionLibrary::editor_run_behavior()`](../init/trait.ExtensionLibrary.html#method.editor_run_behavior)
/// for more information and further customization.
///
/// This behaves similarly to [GDScript's `@tool` feature](https://docs.godotengine.org/en/stable/tutorials/plugins/running_code_in_the_editor.html).
///
/// **Note**: As in GDScript, the class must be marked as a `tool` to be accessible in the editor (e.g., for use by editor plugins and inspectors).
///
/// ## Editor plugins
///
/// Classes inheriting `EditorPlugin` will be automatically instantiated and added
/// to the editor when launched.
///
/// See [Godot's documentation of editor plugins](https://docs.godotengine.org/en/stable/tutorials/plugins/editor/index.html)
/// for more information about editor plugins. But note that you do not need to create and enable the plugin
/// through Godot's `Create New Plugin` menu for it to work, simply creating the class which inherits `EditorPlugin`
/// automatically enables it when the library is loaded.
///
/// This should usually be combined with `#[class(tool)]` so that the code you write will actually run in the
/// editor.
///
/// ### Editor plugins -- hot reload interaction
///
/// During hot reload, Godot firstly unloads `EditorPlugin`s, then changes all alive GDExtension classes instances into their base objects
/// (classes inheriting `Resource` become `Resource`, classes inheriting `Node` become `Node` and so on), then reloads all the classes,
/// and finally changes said instances back into their proper classes.
///
/// `EditorPlugin` will be re-added to the editor before the last step (changing instances from base classes to extension classes) is finished,
/// which might cause issues with loading already cached resources and instantiated nodes.
///
/// In such a case, await one frame until extension is properly hot-reloaded (See: [`godot::task::spawn()`](../task/fn.spawn.html)).
///
/// ## Class renaming
///
/// You may want to have structs with the same name. With Rust, this is allowed using `mod`. However, in GDScript
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
/// If you want to register a class with Godot, but not display in the editor (e.g. when creating a new node), you can use `#[class(internal)]`.
///
/// Classes starting with "Editor" are auto-hidden by Godot. They *must* be marked as internal in godot-rust.
///
/// ```
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(base=Node, init, internal)]
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
///
/// # Documentation
///
/// <div class="stab portability">Available on <strong>crate feature <code>register-docs</code></strong> only.</div>
/// <div class="stab portability">Available on <strong>Godot version <code>4.3+</code></strong> only.</div>
///
/// You can document your functions, classes, members, and signals with the `///` doc comment syntax.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// # #[class(init)]
/// /// This is an example struct for documentation, inside documentation.
/// struct DocumentedStruct {
///     /// This is a class member.
///     /// You can use markdown formatting such as _italics_.
///     ///
///     /// @experimental `@experimental` and `@deprecated` attributes are supported.
///     /// The description for such attribute spans for the whole annotated paragraph.
///     ///
///     /// This is the rest of a doc description.
///     #[var]
///     item: f32,
/// }
///
/// #[godot_api]
/// impl DocumentedStruct {
///     /// This provides the item, after adding `0.2`.
///     #[func]
///     pub fn produce_item(&self) -> f32 {
///         self.item + 0.2
///     }
/// }
/// ```
#[doc(
    alias = "class",
    alias = "base",
    alias = "init",
    alias = "no_init",
    alias = "var",
    alias = "export",
    alias = "tool",
    alias = "rename",
    alias = "internal"
)]
#[proc_macro_derive(
    GodotClass,
    attributes(class, base, hint, var, export, export_group, export_subgroup, init)
)]
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
/// **See sidebar on the left for table of contents.**
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
/// This initializes the `Base<T>` field, and every other field with either `Default::default()` or the value specified in `#[init(val = ...)]`.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode {
///     base: Base<Node>,
///
///     #[init(val = 42)]
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
/// Using _any_ trait other than the one corresponding with the base class will result in compilation failure.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node3D)]
/// pub struct My3DNode;
///
/// #[godot_api]
/// impl INode for My3DNode {
///     fn ready(&mut self) {
///         godot_print!("Hello World!");
///     }
/// }
/// ```
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
///         GString::from("Rust")
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
/// ## RPC attributes
///
/// You can use the `#[rpc]` attribute to let your functions act as remote procedure calls (RPCs) in Godot. This is the Rust equivalent of
/// GDScript's [`@rpc` annotation](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html#remote-procedure-calls).
/// `#[rpc]` is only supported for classes inheriting `Node`, and they need to declare a `Base<T>` field.
///
/// The syntax follows GDScript'a `@rpc`. You can optionally specify up to four keys; omitted ones use their default value.
/// Here's an overview:
///
/// | Setting       | Type             | Possible values (first is default)                 |
/// |---------------|------------------|----------------------------------------------------|
/// | RPC mode      | [`RpcMode`]      | **`authority`**, `any_peer`                        |
/// | Sync          | `bool`           | **`call_remote`**, `call_local`                    |
/// | Transfer mode | [`TransferMode`] | **`unreliable`**, `unreliable_ordered`, `reliable` |
/// | Channel       | `u32`            | any                                                |
///
/// You can also use `#[rpc(config = value)]`, with `value` being an expression of type [`RpcConfig`] in scope, for example a `const` or the
/// call to a function. This can be useful to reuse configurations across multiple RPCs.
///
/// `#[rpc]` implies `#[func]`. You can use both attributes together, if you need to configure other `#[func]`-specific keys.
///
/// For example, the following method declarations are all equivalent:
/// ```no_run
/// # // Polyfill without full codegen.
/// # #![allow(non_camel_case_types)]
/// # #[cfg(not(feature = "codegen-full"))]
/// # enum RpcMode { DISABLED, ANY_PEER, AUTHORITY }
/// # #[cfg(not(feature = "codegen-full"))]
/// # enum TransferMode { UNRELIABLE, UNRELIABLE_ORDERED, RELIABLE }
/// # #[cfg(not(feature = "codegen-full"))]
/// # pub struct RpcConfig { pub rpc_mode: RpcMode, pub transfer_mode: TransferMode, pub call_local: bool, pub channel: u32 }
/// # #[cfg(not(feature = "codegen-full"))]
/// # impl Default for RpcConfig { fn default() -> Self { todo!("never called") } }
/// # #[cfg(feature = "codegen-full")]
/// use godot::classes::multiplayer_api::RpcMode;
/// # #[cfg(feature = "codegen-full")]
/// use godot::classes::multiplayer_peer::TransferMode;
/// use godot::prelude::*;
/// # #[cfg(feature = "codegen-full")]
/// use godot::register::RpcConfig;
///
/// # #[derive(GodotClass)]
/// # #[class(no_init, base=Node)]
/// # struct MyStruct {
/// #     base: Base<Node>,
/// # }
/// #[godot_api]
/// impl MyStruct {
///     #[rpc(unreliable_ordered, channel = 2)]
///     fn with_defaults(&mut self) {}
///
///     #[rpc(authority, unreliable_ordered, call_remote, channel = 2)]
///     fn explicit(&mut self) {}
///
///     #[rpc(config = MY_RPC_CONFIG)]
///     fn external_config_const(&mut self) {}
///
///     #[rpc(config = my_rpc_provider())]
///     fn external_config_fn(&mut self) {}
/// }
///
/// const MY_RPC_CONFIG: RpcConfig = RpcConfig {
///     rpc_mode: RpcMode::AUTHORITY,
///     transfer_mode: TransferMode::UNRELIABLE_ORDERED,
///     call_local: false,
///     channel: 2,
/// };
///
/// fn my_rpc_provider() -> RpcConfig {
///     RpcConfig {
///         transfer_mode: TransferMode::UNRELIABLE_ORDERED,
///         channel: 2,
///         ..Default::default() // only possible in fn, not in const.
///     }
/// }
/// ```
///
// Note: for some reason, the intra-doc links don't work here, despite dev-dependency on godot.
/// [`RpcMode`]: ../classes/multiplayer_api/struct.RpcMode.html
/// [`TransferMode`]: ../classes/multiplayer_peer/struct.TransferMode.html
/// [`RpcConfig`]: ../register/struct.RpcConfig.html
///
/// # Lifecycle functions with custom receivers
///
/// Functions inside I* interface impls, similarly to user-defined `#[func]`s, can be annotated with `#[func(gd_self)]` to use `Gd<Self>` receiver and
/// avoid binding the instance.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode;
///
/// #[godot_api]
/// impl INode for MyNode {
///     #[func(gd_self)]
///     fn ready(this: Gd<Self>) {
///         godot_print!("I'm ready!");
///     }
/// }
/// ```
///
/// Only methods with `self` receiver can be used with `#[func(gd_self)]`:
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode;
///
/// #[godot_api]
/// impl INode for MyNode {
///     #[func(gd_self)]
///     fn init(this: Gd<Self>) -> Self {
///         todo!()
///     }
/// }
/// ```
///
/// Currently, `on_notification` can't be used with `[func(gd_self)]`, either:
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init, base=Node)]
/// pub struct MyNode;
///
/// #[godot_api]
/// impl INode for MyNode {
///     #[func(gd_self)]
///     fn on_notification(this: Gd<Self>, what: ObjectNotification) {
///         todo!()
///     }
/// }
/// ```
///
/// # Signals
///
/// The `#[signal]` attribute declares a Godot signal, which can accept parameters, but not return any value.
/// The procedural macro generates a type-safe API that allows you to connect and emit the signal from Rust.
///
/// ```no_run
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyClass {
///     base: Base<RefCounted>, // necessary for #[signal].
/// }
///
/// #[godot_api]
/// impl MyClass {
///     #[signal]
///     fn some_signal(my_parameter: Gd<Node>);
/// }
/// ```
///
/// The above implements the [`WithSignals`] trait for `MyClass`, which provides the `signals()` method. Through that
/// method, you can access all declared signals in `self.signals().some_signal()` or `gd.signals().some_signal()`. The returned object is
/// of type [`TypedSignal`], which provides further APIs for emitting and connecting, among others.
///
/// A detailed explanation with examples is available in the [book chapter _Registering signals_](https://godot-rust.github.io/book/register/signals.html).
///
/// [`WithSignals`]: ../obj/trait.WithSignals.html
/// [`TypedSignal`]: ../register/struct.TypedSignal.html
///
/// # Constants
///
/// Please refer to [the book](https://godot-rust.github.io/book/register/constants.html).
///
/// # Multiple inherent `impl` blocks
///
/// Just like with regular structs, you can have multiple inherent `impl` blocks. This can be useful for code organization or when you want to generate code from a proc-macro.
/// For implementation reasons, all but one `impl` blocks must have the key `secondary`. There is no difference between implementing all functions in one block or splitting them up between multiple blocks.
/// ```no_run
/// # use godot::prelude::*;
/// # #[derive(GodotClass)]
/// # #[class(init)]
/// # struct MyStruct {
/// #     base: Base<RefCounted>,
/// # }
/// #[godot_api]
/// impl MyStruct {
///     #[func]
///     pub fn one(&self) { }
/// }
///
/// #[godot_api(secondary)]
/// impl MyStruct {
///     #[func]
///     pub fn two(&self) { }
/// }
/// ```
///
/// `#[signal]` and `#[rpc]` attributes are not currently supported in secondary `impl` blocks.
///
///```compile_fail
/// # use godot::prelude::*;
/// # #[derive(GodotClass)]
/// # #[class(init, base=Node)]
/// # pub struct MyNode { base: Base<Node> }
/// # // Without primary `impl` block the compilation will always fail (no matter if #[signal] attribute is present or not)
/// # #[godot_api]
/// # impl MyNode {}
/// #[godot_api(secondary)]
/// impl MyNode {
///     #[signal]
///     fn my_signal();
/// }
/// ```
///
///```compile_fail
/// # use godot::prelude::*;
/// # #[derive(GodotClass)]
/// # #[class(init, base=Node)]
/// # pub struct MyNode { base: Base<Node> }
/// # // Without primary `impl` block the compilation will always fail (no matter if #[rpc] attribute is present or not).
/// # #[godot_api]
/// # impl MyNode {}
/// #[godot_api(secondary)]
/// impl MyNode {
///     #[rpc]
///     fn foo(&mut self) {}
/// }
/// ```
#[doc(
    alias = "func",
    alias = "rpc",
    alias = "virtual",
    alias = "signal",
    alias = "constant",
    alias = "rename",
    alias = "secondary"
)]
#[proc_macro_attribute]
pub fn godot_api(meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, |body| {
        class::attribute_godot_api(TokenStream2::from(meta), body)
    })
}

/// Generates a `Class` -> `dyn Trait` upcasting relation.
///
/// This attribute macro can be applied to `impl MyTrait for MyClass` blocks, where `MyClass` is a `GodotClass`. It will automatically
/// implement [`MyClass: AsDyn<dyn MyTrait>`](../obj/trait.AsDyn.html) for you.
///
/// Establishing this relation allows godot-rust to upcast `MyGodotClass` to `dyn Trait` inside the library's
/// [`DynGd`](../obj/struct.DynGd.html) smart pointer.
///
/// # Code generation
/// Given the following code,
/// ```no_run
/// use godot::prelude::*;
///
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyClass {}
///
/// trait MyTrait {}
///
/// #[godot_dyn]
/// impl MyTrait for MyClass {}
/// ```
/// the macro expands to:
/// ```no_run
/// # use godot::prelude::*;
/// # #[derive(GodotClass)]
/// # #[class(init)]
/// # struct MyClass {}
/// # trait MyTrait {}
/// // impl block remains unchanged...
/// impl MyTrait for MyClass {}
///
/// // ...but a new `impl AsDyn` is added.
/// impl AsDyn<dyn MyTrait> for MyClass {
///     fn dyn_upcast(&self) -> &(dyn MyTrait + 'static) { self }
///     fn dyn_upcast_mut(&mut self) -> &mut (dyn MyTrait + 'static) { self }
/// }
/// ```
///
/// # Orphan rule limitations
/// Since `AsDyn` is always a foreign trait, the `#[godot_dyn]` attribute must be used in the same crate as the Godot class's definition.
/// (Currently, Godot classes cannot be shared from libraries, but this may [change in the future](https://github.com/godot-rust/gdext/issues/951).)
#[proc_macro_attribute]
pub fn godot_dyn(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, class::attribute_godot_dyn)
}

/// Derive macro for [`GodotConvert`](../meta/trait.GodotConvert.html) on structs.
///
/// This derive macro also derives [`ToGodot`](../meta/trait.ToGodot.html) and [`FromGodot`](../meta/trait.FromGodot.html).
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
///
/// assert_eq!(obj.to_godot(), &GString::from("hello!"));
/// ```
///
/// However, it will not work for structs with more than one field, even if that field is zero sized:
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
/// This expects a derived [`GodotConvert`](../meta/trait.GodotConvert.html) implementation, using a manual
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

/// Similar to `#[test]`, but runs a benchmark with Godot.
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
// Used by godot-ffi

/// Creates an initialization block for Wasm.
#[proc_macro]
#[cfg(feature = "experimental-wasm")]
pub fn wasm_declare_init_fn(input: TokenStream) -> TokenStream {
    translate_functional(input, ffi_macros::wasm_declare_init_fn)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

type ParseResult<T> = Result<T, venial::Error>;

/// For `#[derive(...)]` derive macros.
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

/// For `#[proc_macro_attribute]` procedural macros.
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

    let result2 = util::venial_parse_meta(&meta2, self_name, &input2)
        .and_then(transform)
        .unwrap_or_else(|e| e.to_compile_error());

    TokenStream::from(result2)
}

/// For `#[proc_macro]` function-style macros.
#[cfg(feature = "experimental-wasm")]
fn translate_functional<F>(input: TokenStream, transform: F) -> TokenStream
where
    F: FnOnce(TokenStream2) -> ParseResult<TokenStream2>,
{
    let input2 = TokenStream2::from(input);
    let result2 = transform(input2).unwrap_or_else(|e| e.to_compile_error());

    TokenStream::from(result2)
}

/// Returns the index of the key in `keys` (if any) that is present.
fn handle_mutually_exclusive_keys(
    parser: &mut KvParser,
    attribute: &str,
    keys: &[&str],
) -> ParseResult<Option<usize>> {
    let (oks, errs) = keys
        .iter()
        .enumerate()
        .map(|(idx, key)| Ok(parser.handle_alone(key)?.then_some(idx)))
        .partition::<Vec<_>, _>(|result: &ParseResult<Option<usize>>| result.is_ok());

    if !errs.is_empty() {
        return bail!(parser.span(), "{errs:?}");
    }

    let found_idxs = oks
        .into_iter()
        .filter_map(|r| r.unwrap()) // `partition` guarantees that this is `Ok`
        .collect::<Vec<_>>();

    match found_idxs.len() {
        0 => Ok(None),
        1 => Ok(Some(found_idxs[0])),
        _ => {
            let offending_keys = keys
                .iter()
                .enumerate()
                .filter(|(idx, _)| found_idxs.contains(idx));

            bail!(
                parser.span(),
                "{attribute} attribute keys {offending_keys:?} are mutually exclusive"
            )
        }
    }
}
