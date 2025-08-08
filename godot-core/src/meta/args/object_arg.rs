/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use godot_ffi::{ExtVariantType, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::{ClassName, FromGodot, GodotConvert, GodotFfiVariant, GodotType, ToGodot};
use crate::obj::{bounds, Bounds, DynGd, Gd, GodotClass, Inherits, RawGd};
use crate::{obj, sys};

/// Objects that can be passed as arguments to Godot engine functions.
///
/// This trait is implemented for **shared references** in multiple ways:
/// - [`&Gd<T>`][crate::obj::Gd]  to pass objects. Subclasses of `T` are explicitly supported.
/// - [`Option<&Gd<T>>`][Option], to pass optional objects. `None` is mapped to a null argument.
/// - [`Gd::null_arg()`], to pass `null` arguments without using `Option`.
///
/// Note that [`AsObjectArg`] is very similar to the more general [`AsArg`][crate::meta::AsArg] trait. The two may be merged in the future.
///
/// # Nullability
/// <div class="warning">
/// The GDExtension API does not inform about nullability of its function parameters. It is up to you to verify that the arguments you pass
/// are only null when this is allowed. Doing this wrong should be safe, but can lead to the function call failing.
/// </div>
///
/// # Different argument types
/// Currently, the trait requires pass-by-ref, which helps detect accidental cloning when interfacing with Godot APIs. Plus, it is more
/// consistent with the [`AsArg`][crate::meta::AsArg] trait (for strings, but also `AsArg<Gd<T>>` as used in
/// [`Array::push()`][crate::builtin::Array::push] and similar methods).
///
/// The following table lists the possible argument types and how you can pass them. `Gd` is short for `Gd<T>`.
///
/// | Type              | Closest accepted type | How to transform |
/// |-------------------|-----------------------|------------------|
/// | `Gd`              | `&Gd`                 | `&arg`           |
/// | `&Gd`             | `&Gd`                 | `arg`            |
/// | `&mut Gd`         | `&Gd`                 | `&*arg`          |
/// | `Option<Gd>`      | `Option<&Gd>`         | `arg.as_ref()`   |
/// | `Option<&Gd>`     | `Option<&Gd>`         | `arg`            |
/// | `Option<&mut Gd>` | `Option<&Gd>`         | `arg.as_deref()` |
/// | (null literal)    |                       | `Gd::null_arg()` |
#[diagnostic::on_unimplemented(
    message = "Argument of type `{Self}` cannot be passed to an `impl AsObjectArg<{T}>` parameter",
    note = "if you pass by value, consider borrowing instead.",
    note = "see also `AsObjectArg` docs: https://godot-rust.github.io/docs/gdext/master/godot/meta/trait.AsObjectArg.html"
)]
pub trait AsObjectArg<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    #[doc(hidden)]
    fn as_object_arg(&self) -> ObjectArg<T>;

    /// Returns
    #[doc(hidden)]
    fn consume_arg(self) -> ObjectCow<T>;
}

/*
Currently not implemented for values, to be consistent with AsArg for by-ref builtins. The idea is that this can discover patterns like
api.method(refc.clone()), and encourage better performance with api.method(&refc). However, we need to see if there's a notable ergonomic
impact, and consider that for nodes, Gd<T> copies are relatively cheap (no ref-counting). There is also some value in prematurely ending
the lifetime of a Gd<T> by moving out, so it's not accidentally used later.

impl<T, U> AsObjectArg<T> for Gd<U>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
    U: Inherits<T>,
{
    fn as_object_arg(&self) -> ObjectArg<T> {
        <&Gd<U>>::as_object_arg(&self)
    }

    fn consume_arg(self) -> ObjectCow<T> {
        ObjectCow::Owned(self.upcast())
    }
}
*/

impl<T, U> AsObjectArg<T> for &Gd<U>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
    U: Inherits<T>,
{
    fn as_object_arg(&self) -> ObjectArg<T> {
        // SAFETY: In the context where as_object_arg() is called (a Godot engine call), the original Gd is guaranteed to remain valid.
        // This function is not part of the public API.
        unsafe { ObjectArg::from_raw_gd(&self.raw) }
    }

    fn consume_arg(self) -> ObjectCow<T> {
        ObjectCow::Borrowed(self.as_object_arg())
    }
}

impl<T, U, D> AsObjectArg<T> for &DynGd<U, D>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
    U: Inherits<T>,
    D: ?Sized,
{
    fn as_object_arg(&self) -> ObjectArg<T> {
        // Reuse Deref.
        let gd: &Gd<U> = self;
        <&Gd<U>>::as_object_arg(&gd)
    }

    fn consume_arg(self) -> ObjectCow<T> {
        // Reuse Deref.
        let gd: &Gd<U> = self;
        <&Gd<U>>::consume_arg(gd)
    }
}

impl<T, U> AsObjectArg<T> for Option<U>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
    U: AsObjectArg<T>,
{
    fn as_object_arg(&self) -> ObjectArg<T> {
        self.as_ref()
            .map_or_else(ObjectArg::null, AsObjectArg::as_object_arg)
    }

    fn consume_arg(self) -> ObjectCow<T> {
        match self {
            Some(obj) => obj.consume_arg(),
            None => Gd::null_arg().consume_arg(),
        }
    }
}

/*
It's relatively common that Godot APIs return `Option<Gd<T>>` or pass this type in virtual functions. To avoid excessive `as_ref()` calls, we
**could** directly support `&Option<Gd>` in addition to `Option<&Gd>`. However, this is currently not done as it hides nullability,
especially in situations where a return type is directly propagated:
    api(create_obj().as_ref())
    api(&create_obj())
While the first is slightly longer, it looks different from a function create_obj() that returns Gd<T> and thus can never be null.
In some scenarios, it's better to immediately ensure non-null (e.g. through `unwrap()`) instead of propagating nulls to the engine.
It's also quite idiomatic to use as_ref() for inner-option transforms in Rust.

impl<T, U> AsObjectArg<T> for &Option<U>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
    for<'a> &'a U: AsObjectArg<T>,
{
    fn as_object_arg(&self) -> ObjectArg<T> {
        match self {
            Some(obj) => obj.as_object_arg(),
            None => ObjectArg::null(),
        }
    }

    fn consume_arg(self) -> ObjectCow<T> {
        match self {
            Some(obj) => obj.consume_arg(),
            None => Gd::null_arg().consume_arg(),
        }
    }
}
*/

impl<T> AsObjectArg<T> for ObjectNullArg<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    fn as_object_arg(&self) -> ObjectArg<T> {
        ObjectArg::null()
    }

    fn consume_arg(self) -> ObjectCow<T> {
        // Null pointer is safe to borrow.
        ObjectCow::Borrowed(ObjectArg::null())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub struct ObjectNullArg<T>(pub(crate) std::marker::PhantomData<*mut T>);

/// Exists for cases where a value must be moved, and retaining `ObjectArg<T>` may not be possible if the source is owned.
///
/// Only used in default-param extender structs.
#[doc(hidden)]
pub enum ObjectCow<T: GodotClass> {
    Owned(Gd<T>),
    Borrowed(ObjectArg<T>),
}

impl<T> ObjectCow<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    /// Returns the actual `ObjectArg` to be passed to function calls.
    ///
    /// [`ObjectCow`] does not implement [`AsObjectArg<T>`] because a differently-named method is more explicit (fewer errors in codegen),
    /// and because [`AsObjectArg::consume_arg()`] is not meaningful.
    pub fn cow_as_object_arg(&self) -> ObjectArg<T> {
        match self {
            ObjectCow::Owned(gd) => gd.as_object_arg(),
            ObjectCow::Borrowed(obj) => obj.clone(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// View for object arguments passed to the Godot engine. Never owning; must be null or backed by `Gd<T>`.
///
/// Could technically have a lifetime, but this makes the whole calling code more complex, e.g. `type CallSig`. Since usage is quite localized
/// and this type doesn't use `Drop` or is propagated further, this should be fine.
#[derive(Debug)]
#[doc(hidden)]
pub struct ObjectArg<T: GodotClass> {
    // Never dropped since it's just a view; see constructor.
    object_ptr: sys::GDExtensionObjectPtr,
    _marker: std::marker::PhantomData<*mut T>,
}

impl<T> ObjectArg<T>
where
    T: GodotClass,
{
    /// # Safety
    /// The referenced `RawGd` must remain valid for the lifetime of this `ObjectArg`.
    pub unsafe fn from_raw_gd<Derived>(obj: &RawGd<Derived>) -> Self
    where
        Derived: Inherits<T>,
    {
        // Runtime check is necessary, to ensure that object is still alive and has correct runtime type.
        if !obj.is_null() {
            obj.check_rtti("from_raw_gd");
        }

        Self {
            object_ptr: obj.obj_sys(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn null() -> Self {
        Self {
            object_ptr: ptr::null_mut(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn is_null(&self) -> bool {
        self.object_ptr.is_null()
    }
}

// #[derive(Clone)] doesn't seem to get bounds right.
impl<T: GodotClass> Clone for ObjectArg<T> {
    fn clone(&self) -> Self {
        Self {
            object_ptr: self.object_ptr,
            _marker: std::marker::PhantomData,
        }
    }
}

// SAFETY: see impl GodotFfi for RawGd.
unsafe impl<T> GodotFfi for ObjectArg<T>
where
    T: GodotClass,
{
    // If anything changes here, keep in sync with RawGd impl.

    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::OBJECT);

    unsafe fn new_from_sys(_ptr: sys::GDExtensionConstTypePtr) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    unsafe fn new_with_uninit(_init: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    unsafe fn new_with_init(_init: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        self.object_ptr.cast()
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.object_ptr.cast()
    }

    // For more context around `ref_get_object` and `ref_set_object`, see:
    // https://github.com/godotengine/godot-cpp/issues/954

    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        obj::object_as_arg_ptr(&self.object_ptr)
    }

    unsafe fn from_arg_ptr(_ptr: sys::GDExtensionTypePtr, _call_type: PtrcallType) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    unsafe fn move_return_ptr(self, _ptr: sys::GDExtensionTypePtr, _call_type: PtrcallType) {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }
}

impl<T: GodotClass> GodotConvert for ObjectArg<T> {
    type Via = Self;
}

impl<T: GodotClass> ToGodot for ObjectArg<T> {
    type ToVia<'v> = Self;

    fn to_godot(&self) -> Self::ToVia<'_> {
        (*self).clone()
    }
}

// TODO refactor signature tuples into separate in+out traits, so FromGodot is no longer needed.
impl<T: GodotClass> FromGodot for ObjectArg<T> {
    fn try_from_godot(_via: Self::Via) -> Result<Self, ConvertError> {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }
}

impl<T: GodotClass> GodotType for ObjectArg<T> {
    type Ffi = Self;
    type ToFfi<'f> = Self; // TODO: maybe ObjectArg becomes redundant with RefArg?

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        (*self).clone()
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(_ffi: Self::Ffi) -> Result<Self, ConvertError> {
        //unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
        Ok(_ffi)
    }

    fn class_name() -> ClassName {
        T::class_name()
    }

    fn godot_type_name() -> String {
        T::class_name().to_string()
    }
}

impl<T: GodotClass> GodotFfiVariant for ObjectArg<T> {
    fn ffi_to_variant(&self) -> Variant {
        // Note: currently likely not invoked since there are no known varcall APIs taking Object parameters; however this might change.
        obj::object_ffi_to_variant(self)
    }

    fn ffi_from_variant(_variant: &Variant) -> Result<Self, ConvertError> {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }
}

impl<T: GodotClass> GodotNullableFfi for ObjectArg<T> {
    fn null() -> Self {
        Self::null()
    }

    fn is_null(&self) -> bool {
        Self::is_null(self)
    }
}
