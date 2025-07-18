/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use godot_ffi::{ffi_methods, ExtVariantType, GdextBuild, GodotFfi};

use crate::builtin::inner;

use super::{GString, StringName};

/// A pre-parsed scene tree path.
///
/// # Null bytes
///
/// Note that Godot ignores any bytes after a null-byte. This means that for instance `"hello, world!"` and `"hello, world!\0 ignored by Godot"`
/// will be treated as the same string if converted to a `NodePath`.
///
/// # All string types
///
/// | Intended use case | String type                                |
/// |-------------------|--------------------------------------------|
/// | General purpose   | [`GString`][crate::builtin::GString]       |
/// | Interned names    | [`StringName`][crate::builtin::StringName] |
/// | Scene-node paths  | **`NodePath`**                             |
///
/// # Godot docs
///
/// [`NodePath` (stable)](https://docs.godotengine.org/en/stable/classes/class_nodepath.html)
pub struct NodePath {
    opaque: sys::types::OpaqueNodePath,
}

impl NodePath {
    fn from_opaque(opaque: sys::types::OpaqueNodePath) -> Self {
        Self { opaque }
    }

    /// Returns the node name at position `index`.
    ///
    /// If you want to get a property name instead, check out [`get_subname()`][Self::get_subname].
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let path = NodePath::from("../RigidBody2D/Sprite2D");
    /// godot_print!("{}", path.get_name(0)); // ".."
    /// godot_print!("{}", path.get_name(1)); // "RigidBody2D"
    /// godot_print!("{}", path.get_name(2)); // "Sprite"
    /// ```
    ///
    /// # Panics
    /// In Debug mode, if `index` is out of bounds. In Release, a Godot error is generated and the result is unspecified (but safe).
    pub fn get_name(&self, index: usize) -> StringName {
        let inner = self.as_inner();
        let index = index as i64;

        debug_assert!(
            index < inner.get_name_count(),
            "NodePath '{self}': name at index {index} is out of bounds"
        );

        inner.get_name(index)
    }

    /// Returns the node subname (property) at position `index`.
    ///
    /// If you want to get a node name instead, check out [`get_name()`][Self::get_name].
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let path = NodePath::from("Sprite2D:texture:resource_name");
    /// godot_print!("{}", path.get_subname(0)); // "texture"
    /// godot_print!("{}", path.get_subname(1)); // "resource_name"
    /// ```
    ///
    /// # Panics
    /// In Debug mode, if `index` is out of bounds. In Release, a Godot error is generated and the result is unspecified (but safe).
    pub fn get_subname(&self, index: usize) -> StringName {
        let inner = self.as_inner();
        let index = index as i64;

        debug_assert!(
            index < inner.get_subname_count(),
            "NodePath '{self}': subname at index {index} is out of bounds"
        );

        inner.get_subname(index)
    }

    /// Returns the number of node names in the path. Property subnames are not included.
    pub fn get_name_count(&self) -> usize {
        self.as_inner()
            .get_name_count()
            .try_into()
            .expect("Godot name counts are non-negative ints")
    }

    /// Returns the number of property names ("subnames") in the path. Each subname in the node path is listed after a colon character (`:`).
    pub fn get_subname_count(&self) -> usize {
        self.as_inner()
            .get_subname_count()
            .try_into()
            .expect("Godot subname counts are non-negative ints")
    }

    /// Returns the total number of names + subnames.
    ///
    /// This method does not exist in Godot and is provided in Rust for convenience.
    pub fn get_total_count(&self) -> usize {
        self.get_name_count() + self.get_subname_count()
    }

    /// Returns a 32-bit integer hash value representing the string.
    pub fn hash(&self) -> u32 {
        self.as_inner()
            .hash()
            .try_into()
            .expect("Godot hashes are uint32_t")
    }

    /// Returns the range `begin..exclusive_end` as a new `NodePath`.
    ///
    /// The absolute value of `begin` and `exclusive_end` will be clamped to [`get_total_count()`][Self::get_total_count].
    /// So, to express "until the end", you can simply pass a large value for `exclusive_end`, such as `i32::MAX`.
    ///
    /// If either `begin` or `exclusive_end` are negative, they will be relative to the end of the `NodePath`.  \
    /// For example, `path.subpath(0, -2)` is a shorthand for `path.subpath(0, path.get_total_count() - 2)`.
    ///
    /// _Godot equivalent: `slice`_
    ///
    /// # Compatibility
    /// The `slice()` behavior for Godot <= 4.3 is unintuitive, see [#100954](https://github.com/godotengine/godot/pull/100954). godot-rust
    /// automatically changes this to the fixed version for Godot 4.4+, even when used in older versions. So, the behavior is always the same.
    // i32 used because it can be negative and many Godot APIs use this, see https://github.com/godot-rust/gdext/pull/982/files#r1893732978.
    #[cfg(since_api = "4.3")]
    #[doc(alias = "slice")]
    pub fn subpath(&self, begin: i32, exclusive_end: i32) -> NodePath {
        // Polyfill for bug https://github.com/godotengine/godot/pull/100954, fixed in 4.4.
        let begin = if GdextBuild::since_api("4.4") {
            begin
        } else {
            let name_count = self.get_name_count() as i32;
            let subname_count = self.get_subname_count() as i32;
            let total_count = name_count + subname_count;

            let mut begin = begin.clamp(-total_count, total_count);
            if begin < 0 {
                begin += total_count;
            }
            if begin > name_count {
                begin += 1;
            }
            begin
        };

        self.as_inner().slice(begin as i64, exclusive_end as i64)
    }

    crate::meta::declare_arg_method! {
        /// Use as argument for an [`impl AsArg<GString|StringName>`][crate::meta::AsArg] parameter.
        ///
        /// This is a convenient way to convert arguments of similar string types.
        ///
        /// # Example
        /// [`PackedStringArray`][crate::builtin::PackedStringArray] can insert elements using `AsArg<GString>`, so let's pass a `NodePath`:
        /// ```no_run
        /// # use godot::prelude::*;
        /// let node_path = NodePath::from("Node2D/Label");
        ///
        /// let mut array = PackedStringArray::new();
        /// array.push(node_path.arg());
        /// ```
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerNodePath<'_> {
        inner::InnerNodePath::from_outer(self)
    }
}

// SAFETY:
// - `move_return_ptr`
//   Nothing special needs to be done beyond a `std::mem::swap` when returning a NodePath.
//   So we can just use `ffi_methods`.
//
// - `from_arg_ptr`
//   NodePaths are properly initialized through a `from_sys` call, but the ref-count should be
//   incremented as that is the callee's responsibility. Which we do by calling
//   `std::mem::forget(node_path.clone())`.
unsafe impl GodotFfi for NodePath {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::NODE_PATH);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

crate::meta::impl_godot_as_self!(NodePath);

impl_builtin_traits! {
    for NodePath {
        Default => node_path_construct_default;
        Clone => node_path_construct_copy;
        Drop => node_path_destroy;
        Eq => node_path_operator_equal;
        // NodePath provides no < operator.
        Hash;
    }
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = GString::from(self);
        <GString as fmt::Display>::fmt(&string, f)
    }
}

/// Uses literal syntax from GDScript: `^"node_path"`
impl fmt::Debug for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = GString::from(self);
        write!(f, "^\"{string}\"")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion from/into other string-types

impl_rust_string_conv!(NodePath);

impl From<&str> for NodePath {
    fn from(s: &str) -> Self {
        GString::from(s).into()
    }
}

impl From<String> for NodePath {
    fn from(s: String) -> Self {
        GString::from(s).into()
    }
}

impl From<&String> for NodePath {
    fn from(s: &String) -> Self {
        GString::from(s).into()
    }
}

impl From<&GString> for NodePath {
    fn from(string: &GString) -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(node_path_from_string);
                let args = [string.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

impl From<GString> for NodePath {
    /// Converts this `GString` to a `NodePath`.
    ///
    /// This is identical to `NodePath::from(&string)`, and as such there is no performance benefit.
    fn from(string: GString) -> Self {
        Self::from(&string)
    }
}

impl From<&StringName> for NodePath {
    fn from(string_name: &StringName) -> Self {
        Self::from(GString::from(string_name))
    }
}

impl From<StringName> for NodePath {
    /// Converts this `StringName` to a `NodePath`.
    ///
    /// This is identical to `NodePath::from(&string_name)`, and as such there is no performance benefit.
    fn from(string_name: StringName) -> Self {
        Self::from(GString::from(string_name))
    }
}

#[cfg(feature = "serde")]
mod serialize {
    use super::*;
    use serde::de::{Error, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;

    // For "Available on crate feature `serde`" in docs. Cannot be inherited from module. Also does not support #[derive] (e.g. in Vector2).
    #[cfg_attr(published_docs, doc(cfg(feature = "serde")))]
    impl Serialize for NodePath {
        #[inline]
        fn serialize<S>(
            &self,
            serializer: S,
        ) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
        {
            serializer.serialize_newtype_struct("NodePath", &*self.to_string())
        }
    }

    #[cfg_attr(published_docs, doc(cfg(feature = "serde")))]
    impl<'de> Deserialize<'de> for NodePath {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct NodePathVisitor;

            impl<'de> Visitor<'de> for NodePathVisitor {
                type Value = NodePath;

                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    formatter.write_str("a NodePath")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(NodePath::from(s))
                }

                fn visit_newtype_struct<D>(
                    self,
                    deserializer: D,
                ) -> Result<Self::Value, <D as Deserializer<'de>>::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_str(self)
                }
            }

            deserializer.deserialize_newtype_struct("NodePath", NodePathVisitor)
        }
    }
}
