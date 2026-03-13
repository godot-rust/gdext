/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

use godot_ffi::VariantType;

use crate::builtin::{Array, Callable, Dictionary};
use crate::classes::{Object, RefCounted};
use crate::impl_thread_safe_arg;
use crate::meta::{AsArg, Element};
use crate::obj::bounds::{self, DeclEngine, DynMemory, MemRefCounted};
use crate::obj::{Bounds, Gd, GodotClass, Inherits, NewAlloc, NewGd};

/// Marker trait to make a type eligible to be used in `Unique<T>`.
///
/// Until now just `Gd<T>` implements it. Other built-ins must be considered.
trait UniqueType {}

impl<T: GodotClass> UniqueType for Gd<T> {}
impl<K: Element, V: Element> UniqueType for Dictionary<K, V> {}
impl<V: Element> UniqueType for Array<V> {}

/// Makes sure the inner value is unique and can be safely shared across threads.
///
/// No other part of the engine will have access to the inner value and only `Send + Sync` or other `Unique<T>` values can be passed to
/// functions on `T`. Unique also blocks access to `Gd::clone` or to their `InstanceId`.
///
/// `Unique` supports these generally not thread safe types:
/// - [`Gd`]
/// - [`Dictionary`]
/// - [`Array`]
///
#[expect(private_bounds)]
pub struct Unique<T: UniqueType> {
    inner: T,
}

impl<T: GodotClass + NewAlloc> Unique<Gd<T>> {
    pub fn new_alloc() -> Self {
        Self {
            inner: T::new_alloc(),
        }
    }
}

impl<T: GodotClass + NewGd> Unique<Gd<T>> {
    pub fn new_gd() -> Self {
        Self { inner: T::new_gd() }
    }
}

impl<K: Element, V: Element> Unique<Dictionary<K, V>> {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Unique {
            inner: Dictionary::new(),
        }
    }
}

impl<V: Element> Unique<Array<V>> {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Unique {
            inner: Array::new(),
        }
    }
}

/// Recursively check a [`Refcounted`] engine class for unique access (ref-count = 1).
///
/// It only reliably works for engine types as it is safe to assume that they expose all their internal references to other objects via the
/// get_property_list method. The engine uses the same approach to verify resource uniqueness. We can not be certain that custom, user
/// implemented types reliably do so as well.
fn verify_unique_recursive<
    T: GodotClass<Declarer = DeclEngine, Memory = MemRefCounted>
        + Inherits<RefCounted>
        + Inherits<Object>,
>(
    value: Gd<T>,
) -> Option<Gd<T>> {
    match MemRefCounted::get_ref_count(&value.raw) {
        Some(0) => unreachable!(),
        Some(1) => (),
        None | Some(2..) => return None,
    }

    let mut queue = VecDeque::from_iter([value.clone().upcast::<RefCounted>()]);

    while let Some(next) = queue.pop_front() {
        const VAR_TYPE_MAX: i32 = VariantType::MAX.ord;

        let count = MemRefCounted::get_ref_count(&value.raw)?;

        // Sub resources should have a count of 2. One ref for the root-object and one for us.
        if count > 2 {
            return None;
        }

        let obj_value = value.upcast_ref::<Object>();

        // Scripts could hold references to other objects.
        if obj_value.get_script().is_some() {
            return None;
        }

        // Runtime class could be user defined, so it has to match compile-time class.
        if obj_value.get_class() != T::class_id().to_gstring() {
            return None;
        }

        for dict in next.upcast_object().get_property_list().iter_shared() {
            for (_, value) in dict.iter_shared() {
                match value.get_type() {
                    VariantType { ord: ..=-1 } => unreachable!(),
                    VariantType::NIL
                    | VariantType::BOOL
                    | VariantType::INT
                    | VariantType::FLOAT
                    | VariantType::STRING
                    | VariantType::VECTOR2
                    | VariantType::VECTOR2I
                    | VariantType::RECT2
                    | VariantType::RECT2I
                    | VariantType::VECTOR3
                    | VariantType::VECTOR3I
                    | VariantType::TRANSFORM2D
                    | VariantType::VECTOR4
                    | VariantType::VECTOR4I
                    | VariantType::PLANE
                    | VariantType::QUATERNION
                    | VariantType::AABB
                    | VariantType::BASIS
                    | VariantType::TRANSFORM3D
                    | VariantType::PROJECTION
                    | VariantType::COLOR
                    | VariantType::STRING_NAME
                    | VariantType::NODE_PATH
                    | VariantType::SIGNAL
                    | VariantType::RID => continue,

                    VariantType::OBJECT => {
                        let object = value.try_to::<Gd<RefCounted>>();

                        match object {
                            Ok(ref_counted) => queue.push_back(ref_counted),
                            // Object does not inherit RefCounted and is manually managed.
                            Err(_) => return None,
                        }
                    }

                    VariantType::CALLABLE => {
                        let callable = value.try_to::<Callable>();

                        match callable {
                            // Custom Callables are ref-counted, but we don't have access to the count.
                            Ok(callable) if callable.is_custom() => return None,
                            Ok(_) => (),
                            Err(_) => return None,
                        }
                    }

                    // Dictionary and all Array types are ref-counted.
                    VariantType::DICTIONARY
                    | VariantType::ARRAY
                    | VariantType::PACKED_BYTE_ARRAY
                    | VariantType::PACKED_INT32_ARRAY
                    | VariantType::PACKED_INT64_ARRAY
                    | VariantType::PACKED_FLOAT32_ARRAY
                    | VariantType::PACKED_FLOAT64_ARRAY
                    | VariantType::PACKED_STRING_ARRAY
                    | VariantType::PACKED_VECTOR2_ARRAY
                    | VariantType::PACKED_VECTOR3_ARRAY
                    | VariantType::PACKED_COLOR_ARRAY
                    | VariantType::PACKED_VECTOR4_ARRAY => return None,

                    // Everything outside the VariantType range is unreachable.
                    VariantType {
                        ord: VAR_TYPE_MAX..,
                    } => unreachable!(),
                }
            }
        }
    }

    Some(value)
}

impl<T: GodotClass + Bounds<Memory = MemRefCounted>> Unique<Gd<T>> {
    /// Attempts to verify that the provided ref-counted object is in fact unique.
    ///
    /// This might fail if the object is referenced by anything else or any of its internal references are shared with other objects.
    /// Specific reasons for this conversion to fail:
    ///
    /// - Reference counter is > 1.
    /// - Reference count of any property value is > 1.
    /// - Any property value directly inherits from Object (manually managed).
    /// - Any property value is of type Dictionary or any of the Array types.
    /// - Any property is a custom callable.
    /// - Any property fails these checks recursively.
    ///
    /// Since all checks are applied recursively to all objects which are referenced by the given value this conversion can potentially be quite expensive.
    pub fn try_from_ref_counted(value: Gd<T>) -> Option<Self>
    where
        T: GodotClass<Declarer = DeclEngine, Memory = MemRefCounted>
            + Inherits<RefCounted>
            + Inherits<Object>,
    {
        verify_unique_recursive(value).map(|verified| Self { inner: verified })
    }
}

impl<T: GodotClass<Declarer = bounds::DeclEngine>> Unique<Gd<T>> {
    pub fn apply_gd<F: FnOnce(&mut T) + Send + Sync>(&mut self, f: F) {
        f(&mut *self.inner);
    }
}

impl<T: GodotClass<Declarer = bounds::DeclUser>> Unique<Gd<T>> {
    pub fn apply<F: FnOnce(&mut T) + Send + Sync>(&mut self, f: F) {
        f(&mut *self.inner.bind_mut());
    }
}

impl<K: Element, V: Element> Unique<Dictionary<K, V>> {
    pub fn apply<F: FnOnce(&mut Dictionary<K, V>)>(&mut self, f: F) {
        f(&mut self.inner)
    }
}

impl<V: Element> Unique<Array<V>> {
    pub fn apply<F: FnOnce(&mut Array<V>)>(&mut self, f: F) {
        f(&mut self.inner);
    }
}

#[expect(private_bounds)]
impl<T: UniqueType> Unique<T> {
    pub fn share(self) -> T {
        self.inner
    }
}

unsafe impl<T: UniqueType> Send for Unique<T> {}
unsafe impl<T: UniqueType> Sync for Unique<T> {}

impl<T: GodotClass> AsArg<Gd<T>> for Unique<Gd<T>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Gd<T>>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner)
    }
}

impl<T: GodotClass> AsArg<Option<Gd<T>>> for Unique<Gd<T>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Option<Gd<T>>>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(Some(self.inner))
    }
}

impl_thread_safe_arg!([T: UniqueType] Unique<T>);
