/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Extended documentation
//!
//! This highlights a few concepts in the public API of the `godot` crate. They complement information
//! available on the main crate documentation page and the book.
//!
//! ## Type categories
//!
//! Godot is written in C++, which doesn't have the same strict guarantees about safety and
//! mutability that Rust does. As a result, not everything in this crate will look and feel
//! entirely "rusty". See also [Philosophy](https://godot-rust.github.io/book/contribute/philosophy.html).
//!
//! Traits such as `Clone`, `PartialEq` or `PartialOrd` are designed to mirror Godot semantics,
//! except in cases where Rust is stricter (e.g. float ordering). Cloning a type results in the
//! same observable behavior as assignment or parameter-passing of a GDScript variable.
//!
//! We distinguish four different kinds of types:
//!
//! 1. **Value types**: `i64`, `f64`, and mathematical types like
//!    [`Vector2`][crate::builtin::Vector2] and [`Color`][crate::builtin::Color].
//!
//!    These are the simplest to understand and to work with. They implement `Clone` and often
//!    `Copy` as well. They are implemented with the same memory layout as their counterparts in
//!    Godot itself, and typically have public fields. <br><br>
//!
//! 2. **Copy-on-write types**: [`GString`][crate::builtin::GString],
//!    [`StringName`][crate::builtin::StringName], and [`PackedArray`][crate::builtin::PackedArray] types.
//!
//!    These mostly act like value types, similar to Rust's own `Vec`. You can `Clone` them to get
//!    a full copy of the entire object, as you would expect.
//!
//!    Under the hood in Godot, these types are implemented with copy-on-write, so that data can be
//!    shared until one of the copies needs to be modified. However, this performance optimization
//!    is entirely hidden from the API and you don't normally need to worry about it. <br><br>
//!
//! 3. **Reference-counted types**: [`Array`][crate::builtin::Array],
//!    [`VarDictionary`][crate::builtin::VarDictionary], and [`Gd<T>`][crate::obj::Gd] where `T` inherits
//!    from [`RefCounted`][crate::classes::RefCounted].
//!
//!    These types may share their underlying data between multiple instances: changes to one
//!    instance are visible in another. They are conceptually similar to `Rc<RefCell<...>>`.
//!
//!    Since there is no way to prevent or even detect this sharing from Rust, you need to be more
//!    careful when using such types. For example, when iterating over an `Array`, make sure that
//!    it isn't being modified at the same time through another reference.
//!
//!    `Clone::clone()` on these types creates a new reference to the same instance, while
//!    type-specific methods such as [`Array::duplicate_deep()`][crate::builtin::Array::duplicate_deep]
//!    can be used to make actual copies. <br><br>
//!
//! 4. **Manually managed types**: [`Gd<T>`][crate::obj::Gd] where `T` inherits from
//!    [`Object`][crate::classes::Object] but not from [`RefCounted`][crate::classes::RefCounted];
//!    most notably, this includes all `Node` classes.
//!
//!    These also share data, but do not use reference counting to manage their memory. Instead,
//!    you must either hand over ownership to Godot (e.g. by adding a node to the scene tree) or
//!    free them manually using [`Gd::free()`][crate::obj::Gd::free]. <br><br>
//!
//!
//! ## Ergonomics and panics
//!
//! gdext is designed with usage ergonomics in mind, making it viable for fast prototyping.
//! Part of this design means that users should not constantly be forced to write code such as
//! `obj.cast::<T>().unwrap()`. Instead, they can just write `obj.cast::<T>()`, which may panic at runtime.
//!
//! This approach has several advantages:
//! * The code is more concise and less cluttered.
//! * Methods like `cast()` provide very sophisticated panic messages when they fail (e.g. involved
//!   classes), immediately giving you the necessary context for debugging. This is certainly
//!   preferable over a generic `unwrap()`, and in most cases also over a `expect("literal")`.
//! * Usually, such methods panicking indicate bugs in the application. For example, you have a static
//!   scene tree, and you _know_ that a node of certain type and name exists. `get_node_as::<T>("name")`
//!   thus _must_ succeed, or your mental concept is wrong. In other words, there is not much you can
//!   do at runtime to recover from such errors anyway; the code needs to be fixed.
//!
//! Now, there are of course cases where you _do_ want to check certain assumptions dynamically.
//! Imagine a scene tree that is constructed at runtime, e.g. in a game editor.
//! This is why the library provides "overloads" for most of these methods that return `Option` or `Result`.
//! Such methods have more verbose names and highlight the attempt, e.g. `try_cast()`.
//!
//! To help you identify panicking methods, we use the symbol "⚠️" at the beginning of the documentation;
//! this should also appear immediately in the auto-completion of your IDE. Note that this warning sign is
//! not used as a general panic indicator, but particularly for methods which have a `Option`/`Result`-based
//! overload. If you want to know whether and how a method can panic, check if its documentation has a
//! _Panics_ section.
//! <br><br>
//!
//!
//! ## Thread safety
//!
//! [Godot's own thread safety
//! rules](https://docs.godotengine.org/en/latest/tutorials/performance/thread_safe_apis.html)
//! apply. Types in this crate implement (or don't implement) `Send` and `Sync` wherever
//! appropriate, but the Rust compiler cannot check what happens to an object through C++ or
//! GDScript.
//!
//! As a rule of thumb, if you must use threading, prefer to use [Rust threads](https://doc.rust-lang.org/std/thread)
//! over Godot threads.
//!
//! The Cargo feature `experimental-threads` provides experimental support for multithreading. The underlying safety
//! rules are still being worked out, as such you may encounter unsoundness and an unstable API.
//!
//!
//! ## Builtin API Design
//!
//! See also [`godot::builtin`](crate::builtin) module documentation.
//!
//! Our goal is to strive for a middle ground between idiomatic Rust and existing Godot APIs, achieving a decent balance between ergonomics,
//! correctness and performance. We leverage Rust's type system (such as `Option<T>` or `enum`) where it helps expressivity.
//!
//! We have been using a few guiding principles. Those apply to builtins in particular, but some are relevant in other modules, too.
//!
//! ### 1. `Copy` for value types
//!
//! _Value types_ are types with public fields and no hidden state. This includes all geometric types, colors and RIDs.
//!
//! All value types implement the `Copy` trait and thus have no custom `Drop` impl.
//!
//! ### 2. By-value (`self`) vs. by-reference (`&self`) receivers
//!
//! Most `Copy` builtins use by-value receivers. The exception are matrix-like types (e.g., `Basis`, `Transform2D`, `Transform3D`, `Projection`),
//! whose methods operate on `&self` instead. This is close to how the underlying `glam` library handles it.
//!
//! ### 3. `Default` trait only when the default value is common and useful
//!
//! `Default` is deliberately not implemented for every type. Rationale:
//! - For some types, the default representation (as per Godot) does not constitute a useful value. This goes against Rust's [`Default`] docs,
//!   which explicitly mention "A trait for giving a type a _useful_ default value". For example, `Plane()` in GDScript creates a degenerate
//!   plane which cannot participate in geometric operations.
//! - Not providing `Default` makes users double-check if the value they want is indeed what they intended. While it seems convenient, not
//!   having implicit default or "null" values is a design choice of Rust, avoiding the Billion Dollar Mistake. In many situations, `Option` or
//!   [`OnReady`][crate::obj::OnReady] is a better alternative.
//! - For cases where the Godot default is truly desired, we provide an `invalid()` constructor, e.g. `Callable::invalid()` or `Plane::invalid()`.
//!   This makes it explicit that you're constructing a value that first has to be modified before becoming useful. When used in class fields,
//!   `#[init(val = ...)]` can help you initialize such values.
//! - Outside builtins, we do not implement `Gd::default()` for manually managed types, as this makes it very easy to overlook initialization
//!   (e.g. in `#[derive(Default)]`) and leak memory. A `Gd::new_alloc()` is very explicit.
//!
//! ### 4. Prefer explicit conversions over `From` trait
//!
//! `From` is quite popular in Rust, but unlike traits such as `Debug`, the convenience of `From` can come at a cost. Like every feature, adding
//! an `impl From` needs to be justified -- not the other way around: there doesn't need to be a particular reason why it's _not_ added. But
//! there are in fact some trade-offs to consider:
//!
//! 1. `From` next to named conversion methods/constructors adds another way to do things. While it's sometimes good to have choice, multiple
//!    ways to achieve the same has downsides: users wonder if a subtle difference exists, or if all options are in fact identical.
//!    It's unclear which one is the "preferred" option. Recognizing other people's code becomes harder, because there tend to be dialects.
//! 2. It's often a purely stylistic choice, without functional benefits. Someone may want to write `(1, 2).into()` instead of
//!    `Vector2i::new(1, 2)`. This is not strong enough of a reason -- if brevity is of concern, a function `vec2i(1, 2)` does the job better.
//! 3. `From` is less explicit than a named conversion function. If you see `string.to_variant()` or `color.to_hsv()`, you immediately
//!    know the target type. `string.into()` and `color.into()` lose that aspect. Even with `(1, 2).into()`, you'd first have to check whether
//!    `From` is only converting the tuple, or if it _also_ provides an `i32`-to-`f32` cast, thus resulting in `Vector2` instead of `Vector2i`.
//!    This problem doesn't exist with named constructor functions.
//! 4. The `From` trait doesn't play nicely with type inference. If you write `let v = string.to_variant()`, rustc can infer the type of `v`
//!    based on the right-hand expression alone. With `.into()`, you need follow-up code to determine the type, which may or may not work.
//!    Temporarily commenting out such non-local code breaks the declaration line, too. To make matters worse, turbofish `.into::<Type>()` isn't
//!    possible either.
//! 5. Rust itself [requires](https://doc.rust-lang.org/std/convert/trait.From.html#when-to-implement-from) that `From` conversions are
//!    infallible, lossless, value-preserving and obvious. This rules out a lot of scenarios such as `DynGd::to_gd()` (which only maintains
//!    the class part, not trait) or `Color::try_to_hsv()` (which is fallible and lossy).
//!
//! One main reason to support `From` is to allow generic programming, in particular `impl Into<T>` parameters. This is also the reason
//! why the string types have historically implemented the trait. But this became less relevant with the advent of
//! [`AsArg<T>`][crate::meta::AsArg] taking that role, and thus may change in the future.
//!
//! ### 5. `Option` for fallible operations
//!
//! GDScript often uses degenerate types and custom null states to express that an operation isn't successful. This isn't always consistent:
//! - [`Rect2::intersection()`] returns an empty rectangle (i.e. you need to check its size).
//! - [`Plane::intersects_ray()`] returns a `Variant` which is NIL in case of no intersection. While this is a better way to deal with it,
//!   it's not immediately obvious that the result is a point (`Vector2`), and comes with extra marshaling overhead.
//!
//! Rust uses `Option` in such cases, making the error state explicit and preventing that the result is accidentally interpreted as valid.
//!
//! [`Rect2::intersection()`]: https://docs.godotengine.org/en/stable/classes/class_rect2.html#class-rect2-method-intersection
//! [`Plane::intersects_ray()`]: https://docs.godotengine.org/en/stable/classes/class_plane.html#class-plane-method-intersects-ray
//!
//! ### 6. Public fields and soft invariants
//!
//! Some geometric types are subject to "soft invariants". These invariants are not enforced at all times but are essential for certain
//! operations. For example, bounding boxes must have non-negative volume for operations like intersection or containment checks. Planes
//! must have a non-zero normal vector.
//!
//! We cannot make them hard invariants (no invalid value may ever exist), because that would disallow the convenient public fields, and
//! it would also mean every value coming over the FFI boundary (e.g. an `#[export]` field set in UI) would constantly need to be validated
//! and reset to a different "sane" value.
//!
//! For **geometric operations**, Godot often doesn't specify the behavior if values are degenerate, which can propagate bugs that then lead
//! to follow-up problems. godot-rust instead provides best-effort validations _during an operation_, which cause panics if such invalid states
//! are detected (at least in Debug mode). Consult the docs of a concrete type to see its guarantees.
//!
//! ### 7. RIIR for some, but not all builtins
//!
//! Builtins use varying degrees of Rust vs. engine code for their implementations. This may change over time and is generally an implementation
//! detail.
//!
//! - 100% Rust, often supported by the `glam` library:
//!   - all vector types (`Vector2`, `Vector2i`, `Vector3`, `Vector3i`, `Vector4`, `Vector4i`)
//!   - all bounding boxes (`Rect2`, `Rect2i`, `Aabb`)
//!   - 2D/3D matrices (`Basis`, `Transform2D`, `Transform3D`)
//!   - `Plane`
//!   - `Rid` (just an integer)
//! - Partial Rust: `Color`, `Quaternion`, `Projection`
//! - Only Godot FFI: all others (containers, strings, callables, variant, ...)
//!
//! The rationale here is that operations which are absolutely ubiquitous in game development, such as vector/matrix operations, benefit
//! a lot from being directly implemented in Rust. This avoids FFI calls, which aren't necessarily slow, but remove a lot of optimization
//! potential for rustc/LLVM.
//!
//! Other types, that are used less in bulk and less often in performance-critical paths (e.g. `Projection`), partially fall back to Godot APIs.
//! Some operations are reasonably complex to implement in Rust, and we're not a math library, nor do we want to depend on one besides `glam`.
//! An ever-increasing maintenance burden for geometry re-implementations is also detrimental.
//!
//! TLDR: it's a trade-off between performance, maintenance effort and correctness -- the current combination of `glam` and Godot seems to be a
//! relatively well-working sweet spot.
//!
//! ### 8. `glam` types are not exposed in public API
//!
//! While Godot and `glam` share common operations, there are also lots of differences and Godot specific APIs.
//! As a result, godot-rust defines its own vector and matrix types, making `glam` an implementation details.
//!
//! Alternatives considered:
//!
//! 1. Re-export types of an existing vector algebra crate (like `glam`).
//!    The `gdnative` crate started out this way, using types from `euclid`, but [became impractical](https://github.com/godot-rust/gdnative/issues/594#issue-705061720).
//!    Even with extension traits, there would be lots of compromises, where existing and Godot APIs differ slightly.
//!
//!    Furthermore, it would create a strong dependency on a volatile API outside our control. `glam` had 9 SemVer-breaking versions over the
//!    timespan of two years (2022-2024). While it's often easy to migrate and the changes notably improve the library, this would mean that any
//!    breaking change would also become breaking for godot-rust, requiring a SemVer bump. By abstracting this, we can have our own timeline.
//!
//! 2. We could opaquely wrap types, i.e. `Vector2` would contain a private `glam::Vec2`. This would prevent direct field access, which is
//!    _extremely_ inconvenient for vectors. And it would still require us to redefine the front-end of the entire API.
//!
//! Eventually, we might add support for [`mint`](https://crates.io/crates/mint) to allow conversions to other linear algebra libraries in the
//! ecosystem. (Note that `mint` intentionally offers no math operations, see e.g. [mint#75](https://github.com/kvark/mint/issues/75)).
