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
//!    [`StringName`][crate::builtin::StringName], and `Packed*Array` types.
//!
//!    These mostly act like value types, similar to Rust's own `Vec`. You can `Clone` them to get
//!    a full copy of the entire object, as you would expect.
//!
//!    Under the hood in Godot, these types are implemented with copy-on-write, so that data can be
//!    shared until one of the copies needs to be modified. However, this performance optimization
//!    is entirely hidden from the API and you don't normally need to worry about it. <br><br>
//!
//! 3. **Reference-counted types**: [`Array`][crate::builtin::Array],
//!    [`Dictionary`][crate::builtin::Dictionary], and [`Gd<T>`][crate::obj::Gd] where `T` inherits
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
