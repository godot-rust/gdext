/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use godot_ffi as sys;
use sys::{interface_fn, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::meta::{
    ClassName, ConvertError, FromGodot, FromVariantError, GodotConvert, GodotFfiVariant, GodotType,
    ToGodot,
};
use crate::builtin::Variant;
use crate::obj::dom::Domain as _;
use crate::obj::mem::Memory as _;
use crate::obj::GdDerefTarget;
use crate::obj::{dom, GodotClass};
use crate::obj::{GdMut, GdRef, InstanceId};
use crate::storage::InstanceStorage;
use crate::{engine, out};

/// Low-level bindings for object pointers in Godot.
///
/// This should not be used directly, you should either use [`Gd<T>`](super::Gd) or [`Option<Gd<T>>`]
/// depending on whether you need a nullable object pointer or not.
#[repr(C)]
#[doc(hidden)]
pub struct RawGd<T: GodotClass> {
    pub(super) obj: *mut T,
    // Must not be changed after initialization.
    cached_instance_id: Option<InstanceId>,
}

impl<T: GodotClass> RawGd<T> {
    /// Create a new object representing a null in Godot.
    pub(super) fn null() -> Self {
        Self {
            obj: ptr::null_mut(),
            cached_instance_id: None,
        }
    }

    /// Initializes this `RawGd<T>` from the object pointer as a **weak ref**, meaning it does not
    /// initialize/increment the reference counter.
    ///
    /// # Safety
    ///
    /// `obj` must be a valid object pointer or a null pointer.
    pub(super) unsafe fn from_obj_sys_weak(obj: sys::GDExtensionObjectPtr) -> Self {
        let mut instance_id = None;
        if !obj.is_null() {
            let id =
                unsafe { interface_fn!(object_get_instance_id)(obj as sys::GDExtensionObjectPtr) };
            instance_id = InstanceId::try_from_u64(id);
        }

        Self {
            obj: obj as *mut T,
            cached_instance_id: instance_id,
        }
    }

    /// Initializes this `RawGd<T>` from the object pointer as a **strong ref**, meaning it initializes
    /// /increments the reference counter and keeps the object alive.
    ///
    /// This is the default for most initializations from FFI. In cases where reference counter
    /// should explicitly **not** be updated, [`from_obj_sys_weak()`](Self::from_obj_sys_weak) is available.
    ///
    /// # Safety
    ///
    /// `obj` must be a valid object pointer or a null pointer.
    pub(super) unsafe fn from_obj_sys(obj: sys::GDExtensionObjectPtr) -> Self {
        Self::from_obj_sys_weak(obj).with_inc_refcount()
    }

    /// Returns `self` but with initialized ref-count.
    fn with_inc_refcount(self) -> Self {
        // Note: use init_ref and not inc_ref, since this might be the first reference increment.
        // Godot expects RefCounted::init_ref to be called instead of RefCounted::reference in that case.
        // init_ref also doesn't hurt (except 1 possibly unnecessary check).
        T::Mem::maybe_init_ref(&self);
        self
    }

    /// Returns `true` if the object is null.
    ///
    /// This does not check if the object is dead, for that use
    /// [`instance_id_or_none()`](Self::instance_id_or_none).
    pub(crate) fn is_null(&self) -> bool {
        self.obj.is_null() || self.cached_instance_id.is_none()
    }

    pub(crate) fn instance_id_unchecked(&self) -> Option<InstanceId> {
        self.cached_instance_id
    }

    pub(crate) fn is_instance_valid(&self) -> bool {
        self.cached_instance_id
            .map(|id| engine::utilities::is_instance_id_valid(id.to_i64()))
            .unwrap_or(false)
    }

    // See use-site for explanation.
    fn is_cast_valid<U>(&self) -> bool
    where
        U: GodotClass,
    {
        if self.is_null() {
            // Null can be cast to anything.
            return true;
        }

        let as_obj =
            unsafe { self.ffi_cast::<engine::Object>() }.expect("Everything inherits object");
        let cast_is_valid = as_obj
            .as_target()
            .expect("object is not null")
            .is_class(U::class_name().to_godot_string());
        std::mem::forget(as_obj);
        cast_is_valid
    }

    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error
    pub(super) fn owned_cast<U>(self) -> Result<RawGd<U>, Self>
    where
        U: GodotClass,
    {
        // Workaround for bug in Godot 4.0 that makes casts always succeed (https://github.com/godot-rust/gdext/issues/158).
        // TODO once fixed in Godot, use #[cfg(before_api = "4.1")]
        if !self.is_cast_valid::<U>() {
            return Err(self);
        }

        // The unsafe { std::mem::transmute<&T, &Base>(self.inner()) } relies on the C++ static_cast class casts
        // to return the same pointer, however in theory those may yield a different pointer (VTable offset,
        // virtual inheritance etc.). It *seems* to work so far, but this is no indication it's not UB.
        //
        // The Deref/DerefMut impls for T implement an "implicit upcast" on the object (not Gd) level and
        // rely on this (e.g. &Node3D -> &Node).

        let result = unsafe { self.ffi_cast::<U>() };
        match result {
            Some(cast_obj) => {
                // duplicated ref, one must be wiped
                std::mem::forget(self);
                Ok(cast_obj)
            }
            None => Err(self),
        }
    }

    /// # Safety
    /// Does not transfer ownership and is thus unsafe. Also operates on shared ref. Either the parameter or
    /// the return value *must* be forgotten (since reference counts are not updated).
    pub(super) unsafe fn ffi_cast<U>(&self) -> Option<RawGd<U>>
    where
        U: GodotClass,
    {
        // `self` may be null when we convert a null-variant into a `Option<Gd<T>>`. Since we use `ffi_cast`
        // in the `ffi_from_variant` conversion function to ensure type-correctness. So a null-variant would
        // be cast into a null `RawGd<Object>` which is then casted to a null `RawGd<T>` which is then
        // converted into a `None` `Option<Gd<T>>`.
        if self.is_null() {
            // Null can be cast to anything.
            // Forgetting a null doesn't do anything, since dropping a null also does nothing.
            return Some(RawGd::null());
        }

        let class_tag = interface_fn!(classdb_get_class_tag)(U::class_name().string_sys());
        let cast_object_ptr = interface_fn!(object_cast_to)(self.obj_sys(), class_tag);

        // Create weak object, as ownership will be moved and reference-counter stays the same
        sys::ptr_then(cast_object_ptr, |ptr| RawGd::from_obj_sys_weak(ptr))
    }

    pub(crate) fn as_ref_counted<R>(&self, apply: impl Fn(&mut engine::RefCounted) -> R) -> R {
        let tmp = unsafe { self.ffi_cast::<engine::RefCounted>() };
        let mut tmp = tmp.expect("object expected to inherit RefCounted");
        let return_val =
            <engine::RefCounted as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj));

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    pub(crate) fn as_object<R>(&self, apply: impl Fn(&mut engine::Object) -> R) -> R {
        let tmp = unsafe { self.ffi_cast::<engine::Object>() };
        let mut tmp = tmp.expect("object expected to inherit Object; should never fail");
        // let return_val = apply(tmp.inner_mut());
        let return_val =
            <engine::Object as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj));

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    // Target is always an engine class:
    // * if T is an engine class => T
    // * if T is a user class => T::Base
    pub(super) fn as_target(&self) -> Option<&GdDerefTarget<T>> {
        if self.is_null() {
            return None;
        }

        // SAFETY:
        // Every engine object is a struct like
        //
        // #[repr(C)]
        // struct Node3D {
        //     object_ptr: sys::GDExtensionObjectPtr,  // <- pointer
        //     instance_id: InstanceId,                // <- non-zero u64
        // }
        //
        // and `RawGd` looks like
        //
        // #[repr(C)]
        // pub struct RawGd<T: GodotClass> {
        //     pub(super) obj: *mut T,                 // <- pointer
        //     cached_instance_id: Option<InstanceId>, // <- u64
        // }
        //
        // So since self isn't null, that means `cached_instance_id` is not 0, and the two layouts are
        // compatible.
        let target = unsafe {
            std::mem::transmute::<
                &Self,
                &<<T as GodotClass>::Declarer as dom::Domain>::DerefTarget<T>,
            >(self)
        };

        Some(target)
    }

    // Target is always an engine class:
    // * if T is an engine class => T
    // * if T is a user class => T::Base
    pub(super) fn as_target_mut(&mut self) -> Option<&mut GdDerefTarget<T>> {
        if self.is_null() {
            return None;
        }

        // SAFETY: see also `as_target()`
        //
        // We have a mutable reference to self, and thus it's entirely safe to transmute that into a
        // mutable reference to a compatible type.
        let target = unsafe {
            std::mem::transmute::<
                &mut Self,
                &mut <<T as GodotClass>::Declarer as dom::Domain>::DerefTarget<T>,
            >(self)
        };

        Some(target)
    }

    pub(super) fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.obj as sys::GDExtensionObjectPtr
    }
}

impl<T> RawGd<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
    /// Hands out a guard for a shared borrow, through which the user instance can be read.
    ///
    /// See [`crate::obj::Gd::bind()`] for a more in depth explanation.
    // Note: possible names: write/read, hold/hold_mut, r/w, r/rw, ...
    pub(crate) fn bind(&self) -> GdRef<T> {
        engine::ensure_object_alive(self.cached_instance_id, self.obj_sys(), "bind");
        GdRef::from_cell(self.storage().unwrap().get())
    }

    /// Hands out a guard for an exclusive borrow, through which the user instance can be read and written.
    ///
    /// See [`crate::obj::Gd::bind_mut()`] for a more in depth explanation.
    pub(crate) fn bind_mut(&mut self) -> GdMut<T> {
        engine::ensure_object_alive(self.cached_instance_id, self.obj_sys(), "bind_mut");
        GdMut::from_cell(self.storage().unwrap().get_mut())
    }

    /// Storage object associated with the extension instance.
    ///
    /// Returns `None` if self is null.
    pub(crate) fn storage(&self) -> Option<&InstanceStorage<T>> {
        // SAFETY: instance pointer belongs to this instance. We only get a shared reference, no exclusive access, so even
        // calling this from multiple Gd pointers is safe.
        // Potential issue is a concurrent free() in multi-threaded access; but that would need to be guarded against inside free().
        unsafe {
            let binding = self.resolve_instance_ptr();
            sys::ptr_then(binding, |binding| crate::private::as_storage::<T>(binding))
        }
    }

    unsafe fn resolve_instance_ptr(&self) -> sys::GDExtensionClassInstancePtr {
        if self.is_null() {
            return ptr::null_mut();
        }

        let callbacks = crate::storage::nop_instance_callbacks();
        let token = sys::get_library() as *mut std::ffi::c_void;
        let binding = interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);

        debug_assert!(
            !binding.is_null(),
            "Class {} -- null instance; does the class have a Godot creator function?",
            std::any::type_name::<T>()
        );
        binding as sys::GDExtensionClassInstancePtr
    }
}

// SAFETY:
// - `move_return_ptr`
//   When the `call_type` is `PtrcallType::Virtual`, and the current type is known to inherit from `RefCounted`
//   then we use `ref_get_object`. Otherwise we use `Gd::from_obj_sys`.
// - `from_arg_ptr`
//   When the `call_type` is `PtrcallType::Virtual`, and the current type is known to inherit from `RefCounted`
//   then we use `ref_set_object`. Otherwise we use `std::ptr::write`. Finally we forget `self` as we pass
//   ownership to the caller.
unsafe impl<T> GodotFfi for RawGd<T>
where
    T: GodotClass,
{
    fn variant_type() -> sys::VariantType {
        sys::VariantType::Object
    }

    unsafe fn from_sys(ptr: sys::GDExtensionTypePtr) -> Self {
        Self::from_obj_sys_weak(ptr as sys::GDExtensionObjectPtr)
    }

    unsafe fn from_sys_init(init: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
        let obj = raw_object_init(init);
        Self::from_obj_sys_weak(obj)
    }

    fn sys(&self) -> sys::GDExtensionTypePtr {
        self.obj as sys::GDExtensionTypePtr
    }

    // For more context around `ref_get_object` and `ref_set_object`, see:
    // https://github.com/godotengine/godot-cpp/issues/954

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self {
        if ptr.is_null() {
            return Self::null();
        }

        let obj_ptr = if T::Mem::pass_as_ref(call_type) {
            // ptr is `Ref<T>*`
            // See the docs for `PtrcallType::Virtual` for more info on `Ref<T>`.
            interface_fn!(ref_get_object)(ptr as sys::GDExtensionRefPtr)
        } else if cfg!(since_api = "4.1") || matches!(call_type, PtrcallType::Virtual) {
            // ptr is `T**`
            *(ptr as *mut sys::GDExtensionObjectPtr)
        } else {
            // ptr is `T*`
            ptr as sys::GDExtensionObjectPtr
        };

        // obj_ptr is `T*`
        Self::from_obj_sys(obj_ptr)
    }

    unsafe fn move_return_ptr(self, ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) {
        if T::Mem::pass_as_ref(call_type) {
            interface_fn!(ref_set_object)(ptr as sys::GDExtensionRefPtr, self.obj_sys())
        } else {
            ptr::write(ptr as *mut _, self.obj)
        }
        // We've passed ownership to caller.
        std::mem::forget(self);
    }

    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        // We're passing a reference to the object to the callee. If the reference count needs to be
        // incremented then the callee will do so. We do not need to prematurely do so.
        //
        // In Rust terms, if `T` is refcounted then we are effectively passing a `&Arc<T>`, and the callee
        // would need to call `.clone()` if desired.

        // In 4.0, argument pointers are passed to godot as `T*`, except for in virtual method calls. We
        // can't perform virtual method calls currently, so they are always `T*`.
        //
        // In 4.1 argument pointers were standardized to always be `T**`.
        #[cfg(before_api = "4.1")]
        {
            self.sys_const()
        }

        #[cfg(since_api = "4.1")]
        {
            std::ptr::addr_of!(self.obj) as sys::GDExtensionConstTypePtr
        }
    }
}

impl<T: GodotClass> GodotConvert for RawGd<T> {
    type Via = Self;
}

impl<T: GodotClass> ToGodot for RawGd<T> {
    fn to_godot(&self) -> Self::Via {
        self.clone()
    }

    fn into_godot(self) -> Self::Via {
        self
    }
}

impl<T: GodotClass> FromGodot for RawGd<T> {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

impl<T: GodotClass> GodotNullableFfi for RawGd<T> {
    fn flatten_option(opt: Option<Self>) -> Self {
        match opt {
            Some(raw) => raw,
            None => Self::null(),
        }
    }

    fn is_null(&self) -> bool {
        Self::is_null(self)
    }
}

/// Runs `init_fn` on the address of a pointer (initialized to null), then returns that pointer, possibly still null.
///
/// # Safety
/// `init_fn` must be a function that correctly handles a _type pointer_ pointing to an _object pointer_.
#[doc(hidden)]
pub unsafe fn raw_object_init(
    init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr),
) -> sys::GDExtensionObjectPtr {
    // return_ptr has type GDExtensionTypePtr = GDExtensionObjectPtr* = OpaqueObject* = Object**
    // (in other words, the type-ptr contains the _address_ of an object-ptr).
    let mut object_ptr: sys::GDExtensionObjectPtr = ptr::null_mut();
    let return_ptr: *mut sys::GDExtensionObjectPtr = ptr::addr_of_mut!(object_ptr);

    init_fn(return_ptr as sys::GDExtensionUninitializedTypePtr);

    // We don't need to know if Object** is null, but if Object* is null; return_ptr has the address of a local (never null).
    object_ptr
}

/// Destructor with semantics depending on memory strategy.
///
/// * If this `RawGd` smart pointer holds a reference-counted type, this will decrement the reference counter.
///   If this was the last remaining reference, dropping it will invoke `T`'s destructor.
///
/// * If the held object is manually-managed, **nothing happens**.
///   To destroy manually-managed `RawGd` pointers, you need to call [`crate::obj::Gd::free()`].
impl<T: GodotClass> Drop for RawGd<T> {
    fn drop(&mut self) {
        // No-op for manually managed objects

        // out!("RawGd::drop   <{}>", std::any::type_name::<T>());

        // SAFETY: This `Gd` wont be dropped again after this.
        let is_last = unsafe { T::Mem::maybe_dec_ref(self) }; // may drop
        if is_last {
            unsafe {
                interface_fn!(object_destroy)(self.obj_sys());
            }
        }

        /*let st = self.storage();
        out!("    objd;  self={:?}, val={:?}", st as *mut _, st.lifecycle);
        //out!("    objd2; self={:?}, val={:?}", st as *mut _, st.lifecycle);

        // If destruction is triggered by Godot, Storage already knows about it, no need to notify it
        if !self.storage().destroyed_by_godot() {
            let is_last = T::Mem::maybe_dec_ref(&self); // may drop
            if is_last {
                //T::Declarer::destroy(self);
                unsafe {
                    interface_fn!(object_destroy)(self.obj_sys());
                }
            }
        }*/
    }
}

impl<T: GodotClass> Clone for RawGd<T> {
    fn clone(&self) -> Self {
        out!("RawGd::clone");
        if !self.is_null() {
            unsafe { Self::from_obj_sys(self.obj as sys::GDExtensionObjectPtr) }
        } else {
            Self::null()
        }
    }
}

impl<T: GodotClass> GodotType for RawGd<T> {
    type Ffi = Self;

    fn to_ffi(&self) -> Self::Ffi {
        self.clone()
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(ffi)
    }

    fn class_name() -> ClassName {
        T::class_name()
    }

    fn godot_type_name() -> String {
        T::class_name().to_string()
    }
}

impl<T: GodotClass> GodotFfiVariant for RawGd<T> {
    fn ffi_to_variant(&self) -> Variant {
        // The conversion method `object_to_variant` DOES increment the reference-count of the object; so nothing to do here.
        // (This behaves differently in the opposite direction `variant_to_object`.)

        unsafe {
            Variant::from_var_sys_init(|variant_ptr| {
                let converter = sys::builtin_fn!(object_to_variant);

                // Note: this is a special case because of an inconsistency in Godot, where sometimes the equivalency is
                // GDExtensionTypePtr == Object** and sometimes GDExtensionTypePtr == Object*. Here, it is the former, thus extra pointer.
                // Reported at https://github.com/godotengine/godot/issues/61967
                let type_ptr = self.sys();
                converter(
                    variant_ptr,
                    ptr::addr_of!(type_ptr) as sys::GDExtensionTypePtr,
                );
            })
        }
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        let raw = unsafe {
            // TODO(#234) replace Gd::<Object> with Self when Godot stops allowing illegal conversions
            // See https://github.com/godot-rust/gdext/issues/158

            // TODO(uninit) - see if we can use from_sys_init()

            // raw_object_init?
            RawGd::<engine::Object>::from_sys_init(|self_ptr| {
                let converter = sys::builtin_fn!(object_from_variant);
                converter(self_ptr, variant.var_sys());
            })
        };

        raw.with_inc_refcount().owned_cast().map_err(|raw| {
            FromVariantError::WrongClass {
                expected: T::class_name(),
            }
            .into_error(raw)
        })
    }
}

impl<T: GodotClass> std::fmt::Debug for RawGd<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            return write!(f, "{} {{ null obj }}", std::any::type_name::<T>());
        }

        let gd = super::Gd::from_ffi(self.clone());
        write!(f, "{gd:?}")
    }
}
