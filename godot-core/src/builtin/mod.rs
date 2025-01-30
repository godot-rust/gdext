/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Built-in types like `Vector2`, `GString` and `Variant`.
//!
//! Please read the [book chapter](https://godot-rust.github.io/book/godot-api/builtins.html) about builtin types.
//!
//! # API design
//!
//! Our goal is to strive for a middle ground between idiomatic Rust and existing Godot APIs, achieving a decent balance between ergonomics,
//! correctness and performance. We leverage Rust's type system (such as `Option<T>` or `enum`) where it helps expressivity.
//!
//! We have been using a few guiding principles. Those apply to builtins in particular, but some are relevant in other modules, too.
//!
//! ## 1. `Copy` for value types
//!
//! _Value types_ are types with public fields and no hidden state. This includes all geometric types, colors and RIDs.
//!
//! All value types implement the `Copy` trait and thus have no custom `Drop` impl.
//!
//! ## 2. By-value (`self`) vs. by-reference (`&self`) receivers
//!
//! Most `Copy` builtins use by-value receivers. The exception are matrix-like types (e.g., `Basis`, `Transform2D`, `Transform3D`, `Projection`),
//! whose methods operate on `&self` instead. This is close to how the underlying `glam` library handles it.
//!
//! ## 3. `Default` trait only when the default value is common and useful
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
//! ## 4. Prefer explicit conversions over `From` trait
//!
//! `From` is quite popular in Rust, but unlike traits such as `Debug`, the convenience of `From` can come at a cost. Like every feature, adding
//! an `impl From` needs to be justified -- not the other way around: there doesn't need to be a particular reason why it's _not_ added. But
//! there are in fact some trade-offs to consider:
//!
//! 1. `From` next to named conversion methods/constructors adds another way to do things. While it's sometimes good to have choice, multiple
//!    ways to achieve the same has downsides: users wonder if a subtle difference exists, or if all options are in fact identical.
//!    It's unclear which one is the "preferred" option. Recognizing other people's code becomes harder, because there tend to be dialects.
//! 2. It's often a purely stylistic choice, without functional benefits. Someone may want to write `(1, 2).into()` instead of
//!    `Vector2::new(1, 2)`. This is not strong enough of a reason -- if brevity is of concern, a function `vec2(1, 2)` does the job better.
//! 3. `From` is less explicit than a named conversion function. If you see `string.to_variant()` or `color.to_hsv()`, you immediately
//!    know the target type. `string.into()` and `color.into()` lose that aspect. Even with `(1, 2).into()`, you'd first have to check whether
//!    `From` is only converting the tuple, or if it _also_ provides an `i32`-to-`f32` cast, thus resulting in `Vector2` instead of `Vector2i`.
//!    This problem doesn't exist with named constructor functions.
//! 4. The `From` trait doesn't play nicely with type inference. If you write `let v = string.to_variant()`, rustc can infer the type of `v`
//!    based on the right-hand expression alone. With `.into()`, you need follow-up code to determine the type, which may or may not work.
//!    Temporarily commenting out such non-local code breaks the declaration line, too. To make matters worse, turbofish `.into::<Type>()` isn't
//!    possible either.
//! 5. Rust itself [requires](https://doc.rust-lang.org/std/convert/trait.From.html#when-to-implement-from) that `From` conversions are
//!    infallible, lossless, value-preserving and obvious. This rules out a lot of scenarios such as `Basis::to_quaternion()` (which only maintains
//!    the rotation part, not scale) or `Color::try_to_hsv()` (which is fallible and lossy).
//!
//! One main reason to support `From` is to allow generic programming, in particular `impl Into<T>` parameters. This is also the reason
//! why the string types have historically implemented the trait. But this became less relevant with the advent of
//! [`AsArg<T>`][crate::meta::AsArg] taking that role, and thus may change in the future.
//!
//! ## 5. `Option` for fallible operations
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
//! ## 6. Public fields and soft invariants
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
//! ## 7. RIIR for some, but not all builtins
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
//! ## 8. `glam` types are not exposed in public API
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

// Re-export macros.
pub use crate::{array, dict, real, reals, varray};

// Re-export generated enums.
pub use crate::gen::central::global_reexported_enums::{Corner, EulerOrder, Side, VariantOperator};
pub use crate::sys::VariantType;
// Not yet public.
pub(crate) use crate::gen::central::VariantDispatch;

#[doc(hidden)]
pub mod __prelude_reexport {
    use super::*;

    pub use aabb::*;
    pub use basis::*;
    pub use callable::*;
    pub use collections::containers::*;
    pub use color::*;
    pub use color_hsv::*;
    pub use plane::*;
    pub use projection::*;
    pub use quaternion::*;
    pub use real_inner::*;
    pub use rect2::*;
    pub use rect2i::*;
    pub use rid::*;
    pub use signal::*;
    pub use string::{GString, NodePath, StringName};
    pub use transform2d::*;
    pub use transform3d::*;
    pub use variant::*;
    pub use vectors::*;

    pub use super::{EulerOrder, Side, VariantOperator, VariantType};
    pub use crate::{array, dict, real, reals, varray};
}

pub use __prelude_reexport::*;

/// Math-related functions and traits like [`ApproxEq`][math::ApproxEq].
pub mod math;

/// Iterator types for arrays and dictionaries.
pub mod iter {
    pub use super::collections::iterators::*;
}

/// Specialized types related to Godot's various string implementations.
pub mod strings {
    pub use super::string::{
        ExGStringFind, ExGStringSplit, ExStringNameFind, ExStringNameSplit, TransientStringNameOrd,
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

// Modules exporting declarative macros must appear first.
mod macros;

// Other modules
mod aabb;
mod basis;
mod callable;
mod collections;
mod color;
mod color_constants; // After color, so that constants are listed after methods in docs (alphabetic ensures that).
mod color_hsv;
mod plane;
mod projection;
mod quaternion;
mod rect2;
mod rect2i;
mod rid;
mod signal;
mod string;
mod transform2d;
mod transform3d;
mod variant;
mod vectors;

// Rename imports because we re-export a subset of types under same module names.
#[path = "real.rs"]
mod real_inner;

#[doc(hidden)]
pub mod inner {
    pub use crate::gen::builtin_classes::*;
}

pub(crate) fn to_i64(i: usize) -> i64 {
    i.try_into().unwrap()
}

pub(crate) fn to_usize(i: i64) -> usize {
    i.try_into().unwrap()
}

pub(crate) fn to_isize(i: usize) -> isize {
    i.try_into().unwrap()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// #[test] utils for serde

#[cfg(all(test, feature = "serde"))] #[cfg_attr(published_docs, doc(cfg(all(test, feature = "serde"))))]
pub(crate) mod test_utils {
    use serde::{Deserialize, Serialize};

    pub(crate) fn roundtrip<T>(value: &T, expected_json: &str)
    where
        T: for<'a> Deserialize<'a> + Serialize + PartialEq + std::fmt::Debug,
    {
        let json: String = serde_json::to_string(value).unwrap();
        let back: T = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(back, *value, "serde round-trip changes value");
        assert_eq!(
            json, expected_json,
            "value does not conform to expected JSON"
        );
    }
}
