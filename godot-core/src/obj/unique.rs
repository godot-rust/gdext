/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{AnyArray, AnyDictionary, Array, Dictionary, Variant};
use crate::classes::RefCounted;
use crate::meta::sealed::Sealed;
use crate::meta::{AsArg, ByValue, Element, ThreadSafeArgContext, ThreadSafeSealed, ToGodot};
use crate::obj::{Gd, GodotClass, NewAlloc, NewGd, bounds};

/// Marker trait to make a type eligible to be used in `Unique<T>`.
pub trait UniqueType: Sealed {}

impl<T: GodotClass> UniqueType for Gd<T> {}
impl<K: Element, V: Element> UniqueType for Dictionary<K, V> {}
impl<V: Element> UniqueType for Array<V> {}
impl UniqueType for AnyArray {}
impl UniqueType for AnyDictionary {}

/// Makes sure the inner value is unique and can be safely shared across threads.
///
/// No other part of the engine will have access to the inner value and only `Send + Sync` or other `Unique<T>` values can be passed to
/// functions on `T`. Unique also blocks access to `Gd::clone` or to their `InstanceId`.
///
/// `Unique` supports these generally not thread safe types:
/// - [`Gd`]
/// - [`Dictionary`]
/// - [`Array`]
pub struct Unique<T: UniqueType> {
    inner: T,
}

impl<T: UniqueType> Unique<T> {
    #[cfg(feature = "codegen-full")]
    pub(crate) unsafe fn new_unchecked(inner: T) -> Self {
        Self { inner }
    }
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

enum QueueEntry {
    Rc(Gd<RefCounted>),
    Dict(AnyDictionary),
    Array(AnyArray),
}

/// Recursively check a [`Refcounted`] engine class for unique access (ref-count = 1).
///
/// It only reliably works for engine types as it is safe to assume that they expose all their internal references to other objects via the
/// get_property_list method. The engine uses the same approach to verify resource uniqueness. We can not be certain that custom, user
/// implemented types reliably do so as well.
#[cfg(since_api = "4.4")]
fn verify_unique_recursive(
    value: Gd<crate::classes::RefCounted>,
) -> Option<Gd<crate::classes::RefCounted>> {
    use std::collections::{HashSet, VecDeque};

    use crate::builtin::{GString, GodotStringExt};
    use crate::classes::class_db::ApiType;
    use crate::classes::{ClassDb, RefCounted};
    use crate::obj::Singleton as _;
    use crate::sys;

    match value.raw.ref_count() {
        0 => unreachable!(),
        1 => (),
        2.. => {
            return None;
        }
    }

    let class_db = ClassDb::singleton();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from_iter([QueueEntry::Rc(value.clone().upcast::<RefCounted>())]);

    while let Some(next) = queue.pop_front() {
        match next {
            QueueEntry::Rc(next) => {
                // If the insert fails we have already checked the instance and ignore it.
                if !seen.insert(next.instance_id()) {
                    continue;
                }

                // Sub resources should have a count of 2. One ref for the root-object and one for us.
                if next.raw.ref_count() > 2 {
                    return None;
                }

                // Scripts could hold references to other objects.
                if next.get_script().is_some() {
                    return None;
                }

                let api_type = class_db.class_get_api_type(&next.dynamic_class_string());

                // Runtime class could be user defined, so it has to match compile-time class.
                if !matches!(api_type, ApiType::CORE | ApiType::EDITOR) {
                    return None;
                }

                // We must check every property to guarantee that they are not referenced by other objects.
                for dict in next.get_property_list().iter_shared() {
                    let ty = dict.at("type").to::<sys::VariantType>();
                    let object = next.get(&dict.at("name").to::<GString>().to_string_name());

                    match verify_variant_type(ty, object) {
                        VariantTypeResult::Pending(value) => {
                            queue.push_back(value);
                        }

                        VariantTypeResult::Success => (),

                        VariantTypeResult::Failure => {
                            return None;
                        }
                    }
                }
            }

            // We only check dictionaries returned by the engine. These are known to be unique.
            QueueEntry::Dict(dict) => {
                for (key, value) in dict.iter_shared() {
                    match verify_variant_type(key.get_type(), key) {
                        VariantTypeResult::Success => (),
                        VariantTypeResult::Pending(queue_entry) => queue.push_back(queue_entry),
                        VariantTypeResult::Failure => return None,
                    }

                    match verify_variant_type(value.get_type(), value) {
                        VariantTypeResult::Success => (),
                        VariantTypeResult::Pending(queue_entry) => queue.push_back(queue_entry),
                        VariantTypeResult::Failure => return None,
                    }
                }
            }

            // We only check arrays returned by the engine. These are known to be unique.
            QueueEntry::Array(array) => {
                for item in array.iter_shared() {
                    match verify_variant_type(item.get_type(), item) {
                        VariantTypeResult::Success => (),
                        VariantTypeResult::Pending(queue_entry) => queue.push_back(queue_entry),
                        VariantTypeResult::Failure => return None,
                    }
                }
            }
        }
    }

    Some(value)
}

/// Return value for `verify_variant_type`.
#[cfg(since_api = "4.4")]
enum VariantTypeResult {
    Success,
    Pending(QueueEntry),
    Failure,
}

/// Verifies the uniqueness of a variant type + value.
///
/// This is currently only used by [`verify_unique_recursive`].
#[cfg(since_api = "4.4")]
fn verify_variant_type(ty: crate::sys::VariantType, value: Variant) -> VariantTypeResult {
    use crate::builtin::Callable;
    use crate::sys;

    const VAR_TYPE_MAX: i32 = sys::VariantType::MAX.ord;

    match ty {
        sys::VariantType { ord: ..=-1 } => unreachable!(),
        sys::VariantType::NIL
        | sys::VariantType::BOOL
        | sys::VariantType::INT
        | sys::VariantType::FLOAT
        | sys::VariantType::STRING
        | sys::VariantType::VECTOR2
        | sys::VariantType::VECTOR2I
        | sys::VariantType::RECT2
        | sys::VariantType::RECT2I
        | sys::VariantType::VECTOR3
        | sys::VariantType::VECTOR3I
        | sys::VariantType::TRANSFORM2D
        | sys::VariantType::VECTOR4
        | sys::VariantType::VECTOR4I
        | sys::VariantType::PLANE
        | sys::VariantType::QUATERNION
        | sys::VariantType::AABB
        | sys::VariantType::BASIS
        | sys::VariantType::TRANSFORM3D
        | sys::VariantType::PROJECTION
        | sys::VariantType::COLOR
        | sys::VariantType::STRING_NAME
        | sys::VariantType::NODE_PATH
        | sys::VariantType::SIGNAL
        | sys::VariantType::RID => VariantTypeResult::Success,

        sys::VariantType::OBJECT => {
            // ignore empty properties
            if value.is_nil() {
                return VariantTypeResult::Success;
            }

            let maybe_ref_counted = value.try_to::<Gd<RefCounted>>();

            match maybe_ref_counted {
                Ok(ref_counted) => VariantTypeResult::Pending(QueueEntry::Rc(ref_counted)),
                // Object does not inherit RefCounted and is manually managed. We don't know how many references to it exist.
                Err(_) => VariantTypeResult::Failure,
            }
        }

        sys::VariantType::CALLABLE => {
            let callable = value.try_to::<Callable>();

            match callable {
                // Custom callables are ref-counted, but we don't have access to the count.
                Ok(callable) if callable.is_custom() => VariantTypeResult::Failure,
                Ok(_) => VariantTypeResult::Success,

                // Value is not a valid callable
                Err(_) => VariantTypeResult::Success,
            }
        }

        // Dictionary and all Array types are ref-counted and we don't have access to the count, but we know that builtin classes return
        // unique dictionaries and arrays.
        sys::VariantType::DICTIONARY => VariantTypeResult::Pending(QueueEntry::Dict(value.to())),
        sys::VariantType::ARRAY => VariantTypeResult::Pending(QueueEntry::Array(value.to())),

        // Packed arrays are not thread safe! But review of various engine classes has shown that the engine always returns
        // a new instance of a packed array from its getter functions. We are currently making the assumption that this holds true for all engine APIs.
        // Since this function only operates on builtin classes we accept the trade off of making this assumption without definitive proof at runtime.
        sys::VariantType::PACKED_BYTE_ARRAY
        | sys::VariantType::PACKED_INT32_ARRAY
        | sys::VariantType::PACKED_INT64_ARRAY
        | sys::VariantType::PACKED_FLOAT32_ARRAY
        | sys::VariantType::PACKED_FLOAT64_ARRAY
        | sys::VariantType::PACKED_STRING_ARRAY
        | sys::VariantType::PACKED_VECTOR2_ARRAY
        | sys::VariantType::PACKED_VECTOR3_ARRAY
        | sys::VariantType::PACKED_COLOR_ARRAY
        | sys::VariantType::PACKED_VECTOR4_ARRAY => VariantTypeResult::Success,

        // Everything outside the VariantType range is unreachable.
        sys::VariantType {
            ord: VAR_TYPE_MAX..,
        } => unreachable!(),
    }
}

#[cfg(since_api = "4.4")]
impl<T: GodotClass> Unique<Gd<T>>
where
    T: crate::obj::Bounds<Memory = bounds::MemRefCounted, Declarer = bounds::DeclEngine>,
    T: crate::obj::Inherits<crate::classes::RefCounted>,
{
    /// Attempts to verify that the provided ref-counted object is in fact unique.
    ///
    /// This is only available for engine classes and they can not have a scripts attached or hold shared references to other resources.
    /// The check is deliberately conservative and errs on the side of not-unique when its not possible to be certain about the uniqueness of a value.
    ///
    /// Specific reasons for this conversion to fail:
    ///
    /// - Object class is user created.
    /// - Object has a script attached.
    /// - Reference counter is > 1.
    /// - Reference count of any property value is > 1.
    /// - Any property value directly inherits from Object (manually managed).
    /// - Any property is a custom callable.
    /// - Any property value fails these checks recursively.
    ///
    /// Since all checks are applied recursively to all objects which are referenced by the given value this conversion is quite expensive.
    pub fn try_from_ref_counted(value: Gd<T>) -> Option<Self> {
        verify_unique_recursive(value.upcast()).map(|verified| Self {
            inner: verified.cast(),
        })
    }
}

impl<T: GodotClass<Declarer = bounds::DeclEngine>> Unique<Gd<T>> {
    /// Apply modifications to unique instance of the inner type.
    ///
    /// All captured outer variables must be thread-safe.
    pub fn apply_gd<F: FnOnce(&mut T) + Send + Sync>(&mut self, f: F) {
        f(&mut *self.inner);
    }
}

impl<T: GodotClass<Declarer = bounds::DeclUser>> Unique<Gd<T>> {
    /// Apply modifications to unique instance of the inner type.
    ///
    /// All captured outer variables must be thread-safe.
    pub fn apply<F: FnOnce(&mut T) + Send + Sync>(&mut self, f: F) {
        f(&mut *self.inner.bind_mut());
    }
}

impl<K: Element, V: Element> Unique<Dictionary<K, V>> {
    /// Apply modifications to unique instance of the inner type.
    ///
    /// All captured outer variables must be thread-safe.
    pub fn apply<F: FnOnce(&mut Dictionary<K, V>) + Send + Sync>(&mut self, f: F) {
        f(&mut self.inner)
    }

    /// Downgrades this typed/untyped `Unique<Dictionary<T>>` into a `Unique<AnyArray>`.
    ///
    /// This method is useful if you need `AnyArray` instead of a typed one.
    pub fn upcast_any_dictionary(self) -> Unique<AnyDictionary> {
        Unique {
            inner: self.inner.upcast_any_dictionary(),
        }
    }
}

impl<V: Element> Unique<Array<V>> {
    /// Apply modifications to unique instance of the inner type.
    ///
    /// All captured outer variables must be thread-safe.
    pub fn apply<F: FnOnce(&mut Array<V>) + Send + Sync>(&mut self, f: F) {
        f(&mut self.inner);
    }

    /// Downgrades this typed/untyped `Unique<Array<T>>` into a `Unique<AnyArray>`.
    ///
    /// This method is useful if you need `AnyArray` instead of a typed one.
    pub fn upcast_any_array(self) -> Unique<AnyArray> {
        Unique {
            inner: self.inner.upcast_any_array(),
        }
    }
}

impl<V: Element + Send + Sync> Array<V> {
    /// Create a unique copy of the array if all elements are thread-safe.
    ///
    /// Arrays are reference counted and could be accessed by multiple threads at the same time. By creating a shallow copy of the array
    /// it can be guaranteed that there are no other references to the array.
    pub fn to_unique(&self) -> Unique<Self> {
        Unique {
            inner: self.duplicate_shallow(),
        }
    }
}

impl<T: UniqueType> Unique<T> {
    /// Unwraps the inner value and gives up all uniqueness guarantees.
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

impl<T: GodotClass> AsArg<Variant> for Unique<Gd<T>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Variant>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner.to_variant())
    }
}

impl<K: Element, V: Element> AsArg<Dictionary<K, V>> for Unique<Dictionary<K, V>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Dictionary<K, V>>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner)
    }
}

impl<K: Element, V: Element> AsArg<AnyDictionary> for Unique<Dictionary<K, V>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, AnyDictionary>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner.upcast_any_dictionary())
    }
}

impl AsArg<AnyDictionary> for Unique<AnyDictionary> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, AnyDictionary>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner)
    }
}

impl<K: Element, V: Element> AsArg<Variant> for Unique<Dictionary<K, V>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Variant>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner.to_variant())
    }
}

impl<V: Element> AsArg<Array<V>> for Unique<Array<V>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Array<V>>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner)
    }
}

impl<V: Element> AsArg<AnyArray> for Unique<Array<V>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, AnyArray>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner.upcast_any_array())
    }
}

impl AsArg<AnyArray> for Unique<AnyArray> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, AnyArray>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner)
    }
}

impl<V: Element> AsArg<Variant> for Unique<Array<V>> {
    fn into_arg<'arg>(self) -> crate::meta::CowArg<'arg, Variant>
    where
        Self: 'arg,
    {
        crate::meta::CowArg::Owned(self.inner.to_variant())
    }
}

impl<I> FromIterator<I> for Unique<Array<I>>
where
    I: Element + ToGodot<Pass = ByValue> + Send + Sync,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut array = Unique::<Array<I>>::new();

        for item in iter {
            array.apply(|array| {
                array.push(item);
            })
        }

        array
    }
}

impl<I: Element, T: UniqueType> FromIterator<Unique<T>> for Unique<Array<I>>
where
    Unique<T>: AsArg<I>,
{
    fn from_iter<Iter: IntoIterator<Item = Unique<T>>>(iter: Iter) -> Self {
        let mut array = Unique::<Array<I>>::new();

        for item in iter {
            array.apply(|array| {
                array.push(item);
            });
        }

        array
    }
}

impl<T: UniqueType> ThreadSafeSealed for Unique<T> {}

impl<T: UniqueType> ThreadSafeArgContext for Unique<T> {
    fn guarantee_thread_safe(&self) {}
}
