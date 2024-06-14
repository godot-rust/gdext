/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use godot_ffi as sys;
use sys::{interface_fn, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::Variant;
use crate::meta::error::{ConvertError, FromVariantError};
use crate::meta::{
    CallContext, ClassName, FromGodot, GodotConvert, GodotFfiVariant, GodotType, ToGodot,
};
use crate::obj::bounds::DynMemory as _;
use crate::obj::rtti::ObjectRtti;
use crate::obj::{bounds, Bounds, GdDerefTarget, GdMut, GdRef, GodotClass, InstanceId};
use crate::storage::{InstanceStorage, Storage};
use crate::{classes, global, out};

/// Low-level bindings for object pointers in Godot.
///
/// This should not be used directly, you should either use [`Gd<T>`](super::Gd) or [`Option<Gd<T>>`]
/// depending on whether you need a nullable object pointer or not.
#[repr(C)]
#[doc(hidden)]
pub struct RawGd<T: GodotClass> {
    pub(super) obj: *mut T,
    // Must not be changed after initialization.
    cached_rtti: Option<ObjectRtti>,
}

impl<T: GodotClass> RawGd<T> {
    /// Create a new object representing a null in Godot.
    pub(super) fn null() -> Self {
        Self {
            obj: ptr::null_mut(),
            cached_rtti: None,
        }
    }

    /// Initializes this `RawGd<T>` from the object pointer as a **weak ref**, meaning it does not
    /// initialize/increment the reference counter.
    ///
    /// # Safety
    ///
    /// `obj` must be a valid object pointer or a null pointer.
    pub(super) unsafe fn from_obj_sys_weak(obj: sys::GDExtensionObjectPtr) -> Self {
        let rtti = if obj.is_null() {
            None
        } else {
            let raw_id = unsafe { interface_fn!(object_get_instance_id)(obj) };

            let instance_id = InstanceId::try_from_u64(raw_id)
                .expect("constructed RawGd weak pointer with instance ID 0");

            // TODO(bromeon): this should query dynamic type of object, which can be different from T (upcast, FromGodot, etc).
            // See comment in ObjectRtti.
            Some(ObjectRtti::of::<T>(instance_id))
        };

        Self {
            obj: obj.cast::<T>(),
            cached_rtti: rtti,
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
    fn with_inc_refcount(mut self) -> Self {
        // Note: use init_ref and not inc_ref, since this might be the first reference increment.
        // Godot expects RefCounted::init_ref to be called instead of RefCounted::reference in that case.
        // init_ref also doesn't hurt (except 1 possibly unnecessary check).
        T::DynMemory::maybe_init_ref(&mut self);
        self
    }

    /// Returns `true` if the object is null.
    ///
    /// This does not check if the object is dead, for that use
    /// [`instance_id_or_none()`](Self::instance_id_or_none).
    pub(crate) fn is_null(&self) -> bool {
        self.obj.is_null() || self.cached_rtti.is_none()
    }

    pub(crate) fn instance_id_unchecked(&self) -> Option<InstanceId> {
        self.cached_rtti.as_ref().map(|rtti| rtti.instance_id())
    }

    pub(crate) fn is_instance_valid(&self) -> bool {
        self.cached_rtti
            .as_ref()
            .map(|rtti| global::is_instance_id_valid(rtti.instance_id().to_i64()))
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

        // SAFETY: object is forgotten below.
        let as_obj =
            unsafe { self.ffi_cast::<classes::Object>() }.expect("everything inherits Object");

        // SAFETY: Object is always a base class.
        let cast_is_valid = unsafe { as_obj.as_upcast_ref::<classes::Object>() }
            .is_class(U::class_name().to_gstring());

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
        // `self` may be null when we convert a null-variant into a `Option<Gd<T>>`, since we use `ffi_cast`
        // in the `ffi_from_variant` conversion function to ensure type-correctness. So the chain would be as follows:
        // - Variant::nil()
        // - null RawGd<Object>
        // - null RawGd<T>
        // - Option::<Gd<T>>::None
        if self.is_null() {
            // Null can be cast to anything.
            // Forgetting a null doesn't do anything, since dropping a null also does nothing.
            return Some(RawGd::null());
        }

        // Before Godot API calls, make sure the object is alive (and in Debug mode, of the correct type).
        // Current design decision: EVERY cast fails on incorrect type, even if target type is correct. This avoids the risk of violated
        // invariants that leak to the Godot implementation. Also, we do not provide a way to recover from bad types -- this is always
        // a bug that must be solved by the user.
        self.check_rtti("ffi_cast");

        let class_tag = interface_fn!(classdb_get_class_tag)(U::class_name().string_sys());
        let cast_object_ptr = interface_fn!(object_cast_to)(self.obj_sys(), class_tag);

        // Create weak object, as ownership will be moved and reference-counter stays the same.
        sys::ptr_then(cast_object_ptr, |ptr| RawGd::from_obj_sys_weak(ptr))
    }

    pub(crate) fn with_ref_counted<R>(&self, apply: impl Fn(&mut classes::RefCounted) -> R) -> R {
        // Note: this previously called Declarer::scoped_mut() - however, no need to go through bind() for changes in base RefCounted.
        // Any accesses to user objects (e.g. destruction if refc=0) would bind anyway.

        let tmp = unsafe { self.ffi_cast::<classes::RefCounted>() };
        let mut tmp = tmp.expect("object expected to inherit RefCounted");
        let return_val = apply(tmp.as_target_mut());

        std::mem::forget(tmp); // no ownership transfer
        return_val
    }

    // TODO replace the above with this -- last time caused UB; investigate.
    // pub(crate) unsafe fn as_ref_counted_unchecked(&mut self) -> &mut classes::RefCounted {
    //     self.as_target_mut()
    // }

    pub(crate) fn as_object(&self) -> &classes::Object {
        // SAFETY: Object is always a valid upcast target.
        unsafe { self.as_upcast_ref() }
    }

    /// # Panics
    /// If this `RawGd` is null. In Debug mode, sanity checks (valid upcast, ID comparisons) can also lead to panics.
    ///
    /// # Safety
    /// - `Base` must actually be a base class of `T`.
    /// - `Base` must be an engine class.
    ///
    /// This is not done via bounds because that would infect all APIs with `Inherits<T>` and leads to cycles in `Deref`.
    /// Bounds should be added on user-facing safe APIs.
    pub(super) unsafe fn as_upcast_ref<Base>(&self) -> &Base
    where
        Base: GodotClass,
    {
        self.ensure_valid_upcast::<Base>();

        // SAFETY:
        // Every engine object is a struct like:
        //
        // #[repr(C)]
        // struct Node3D {
        //     object_ptr: sys::GDExtensionObjectPtr,
        //     rtti: Option<ObjectRtti>,
        // }
        //
        // and `RawGd` looks like:
        //
        // #[repr(C)]
        // pub struct RawGd<T: GodotClass> {
        //     obj: *mut T,
        //     cached_rtti: Option<ObjectRtti>,
        // }
        //
        // The pointers have the same meaning despite different types, and so the whole struct is layout-compatible.
        // In addition, Gd<T> as opposed to RawGd<T> will have the Option always set to Some.
        std::mem::transmute::<&Self, &Base>(self)
    }

    /// # Panics
    /// If this `RawGd` is null. In Debug mode, sanity checks (valid upcast, ID comparisons) can also lead to panics.
    ///
    /// # Safety
    /// - `Base` must actually be a base class of `T`.
    /// - `Base` must be an engine class.
    ///
    /// This is not done via bounds because that would infect all APIs with `Inherits<T>` and leads to cycles in `Deref`.
    /// Bounds should be added on user-facing safe APIs.
    pub(super) unsafe fn as_upcast_mut<Base>(&mut self) -> &mut Base
    where
        Base: GodotClass,
    {
        self.ensure_valid_upcast::<Base>();

        // SAFETY: see also `as_upcast_ref()`.
        //
        // We have a mutable reference to self, and thus it's safe to transmute that into a
        // mutable reference to a compatible type.
        //
        // There cannot be aliasing on the same internal Base object, as every Gd<T> has a different such object -- aliasing
        // of the internal object would thus require multiple &mut Gd<T>, which cannot happen.
        std::mem::transmute::<&mut Self, &mut Base>(self)
    }

    /// # Panics
    /// If this `RawGd` is null.
    pub(super) fn as_target(&self) -> &GdDerefTarget<T> {
        // SAFETY: There are two possible Declarer::DerefTarget types:
        // - T, if T is an engine class
        // - T::Base, if T is a user class
        // Both are valid targets for upcast. And both are always engine types.
        unsafe { self.as_upcast_ref::<GdDerefTarget<T>>() }
    }

    /// # Panics
    /// If this `RawGd` is null.
    pub(super) fn as_target_mut(&mut self) -> &mut GdDerefTarget<T> {
        // SAFETY: See as_target().
        unsafe { self.as_upcast_mut::<GdDerefTarget<T>>() }
    }

    // Clippy believes the type parameters are not used, however they are used in the `.ffi_cast::<Base>` call.
    #[allow(clippy::extra_unused_type_parameters)]
    fn ensure_valid_upcast<Base>(&self)
    where
        Base: GodotClass,
    {
        // Validation object identity.
        self.check_rtti("upcast_ref");
        debug_assert!(!self.is_null(), "cannot upcast null object refs");

        // In Debug builds, go the long path via Godot FFI to verify the results are the same.
        #[cfg(debug_assertions)]
        {
            // SAFETY: we forget the object below and do not leave the function before.
            let ffi_ref: RawGd<Base> =
                unsafe { self.ffi_cast::<Base>().expect("failed FFI upcast") };

            // The ID check is not that expressive; we should do a complete comparison of the ObjectRtti, but currently the dynamic types can
            // be different (see comment in ObjectRtti struct). This at least checks that the transmuted object is not complete garbage.
            // We get direct_id from Self and not Base because the latter has no API with current bounds; but this equivalence is tested in Deref.
            let direct_id = self.instance_id_unchecked().expect("direct_id null");
            let ffi_id = ffi_ref.instance_id_unchecked().expect("ffi_id null");

            assert_eq!(
                direct_id, ffi_id,
                "upcast_ref: direct and FFI IDs differ. This is a bug, please report to gdext maintainers."
            );

            std::mem::forget(ffi_ref);
        }
    }

    /// Verify that the object is non-null and alive. In Debug mode, additionally verify that it is of type `T` or derived.
    pub(crate) fn check_rtti(&self, method_name: &'static str) {
        let call_ctx = CallContext::gd::<T>(method_name);

        let instance_id = self.check_dynamic_type(&call_ctx);
        classes::ensure_object_alive(instance_id, self.obj_sys(), &call_ctx);
    }

    /// Checks only type, not alive-ness. Used in Gd<T> in case of `free()`.
    pub(crate) fn check_dynamic_type(&self, call_ctx: &CallContext<'static>) -> InstanceId {
        debug_assert!(
            !self.is_null(),
            "{call_ctx}: cannot call method on null object",
        );

        let rtti = self.cached_rtti.as_ref();

        // SAFETY: code surrounding RawGd<T> ensures that `self` is non-null; above is just a sanity check against internal bugs.
        let rtti = unsafe { rtti.unwrap_unchecked() };
        rtti.check_type::<T>()
    }

    pub(super) fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.obj as sys::GDExtensionObjectPtr
    }

    pub(super) fn script_sys(&self) -> sys::GDExtensionScriptLanguagePtr
    where
        T: super::Inherits<crate::classes::ScriptLanguage>,
    {
        self.obj.cast()
    }
}

impl<T> RawGd<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    /// Hands out a guard for a shared borrow, through which the user instance can be read.
    ///
    /// See [`crate::obj::Gd::bind()`] for a more in depth explanation.
    // Note: possible names: write/read, hold/hold_mut, r/w, r/rw, ...
    pub(crate) fn bind(&self) -> GdRef<T> {
        self.check_rtti("bind");
        GdRef::from_guard(self.storage().unwrap().get())
    }

    /// Hands out a guard for an exclusive borrow, through which the user instance can be read and written.
    ///
    /// See [`crate::obj::Gd::bind_mut()`] for a more in depth explanation.
    pub(crate) fn bind_mut(&mut self) -> GdMut<T> {
        self.check_rtti("bind_mut");
        GdMut::from_guard(self.storage().unwrap().get_mut())
    }

    /// Storage object associated with the extension instance.
    ///
    /// Returns `None` if self is null.
    pub(crate) fn storage(&self) -> Option<&InstanceStorage<T>> {
        // SAFETY:
        // - We have a `&self`, so the storage must already have been created.
        // - The storage cannot be destroyed while we have a `&self` reference, so it will not be
        //   destroyed for the duration of `'a`.
        unsafe { self.storage_unbounded() }
    }

    /// Storage object associated with the extension instance.
    ///
    /// Returns `None` if self is null.
    ///
    /// # Safety
    ///
    /// This method provides a reference to the storage with an arbitrarily long lifetime `'b`. The reference
    /// must actually be live for the duration of this lifetime.
    ///
    /// The only time when a `&mut` reference can be taken to a `InstanceStorage` is when it is constructed
    /// or destroyed. So it is sufficient to ensure that the storage is not created or destroyed during the
    /// lifetime `'b`.
    pub(crate) unsafe fn storage_unbounded<'b>(&self) -> Option<&'b InstanceStorage<T>> {
        // SAFETY: instance pointer belongs to this instance. We only get a shared reference, no exclusive access, so even
        // calling this from multiple Gd pointers is safe.
        //
        // The caller is responsible for ensuring that the storage is live for the duration of the lifetime
        // `'b`.
        //
        // Potential issue is a concurrent free() in multi-threaded access; but that would need to be guarded against inside free().
        unsafe {
            let binding = self.resolve_instance_ptr();
            sys::ptr_then(binding, |binding| crate::private::as_storage::<T>(binding))
        }
    }

    // TODO: document unsafety in this function, and double check that it actually needs to be unsafe.
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
        sys::VariantType::OBJECT
    }

    unsafe fn new_from_sys(ptr: sys::GDExtensionConstTypePtr) -> Self {
        Self::from_obj_sys_weak(ptr as sys::GDExtensionObjectPtr)
    }

    unsafe fn new_with_uninit(init: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
        let obj = raw_object_init(init);
        Self::from_obj_sys_weak(obj)
    }

    unsafe fn new_with_init(init: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        // `new_with_uninit` creates an initialized pointer.
        Self::new_with_uninit(|return_ptr| init(sys::SysPtr::force_init(return_ptr)))
    }

    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        self.obj.cast()
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.obj.cast()
    }

    // For more context around `ref_get_object` and `ref_set_object`, see:
    // https://github.com/godotengine/godot-cpp/issues/954

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self {
        if ptr.is_null() {
            return Self::null();
        }

        let obj_ptr = if T::DynMemory::pass_as_ref(call_type) {
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
        if T::DynMemory::pass_as_ref(call_type) {
            interface_fn!(ref_set_object)(ptr as sys::GDExtensionRefPtr, self.obj_sys())
        } else {
            ptr::write(ptr as *mut _, self.obj)
        }
        // We've passed ownership to caller.
        std::mem::forget(self);
    }

    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        // No need to call self.check_rtti("as_arg_ptr") here, since this is already done in ToGodot impl.

        // We pass an object to a Godot API. If the reference count needs to be incremented, then the callee (Godot C++ function) will do so.
        // We do not need to prematurely do so. In Rust terms, if `T` is ref-counted, then we are effectively passing a `&Arc<T>`, and the
        // callee would need to invoke `.clone()` if desired.

        // In 4.0, argument pointers are passed to godot as `T*`, except for in virtual method calls. We can't perform virtual method calls
        // currently, so they are always `T*`.
        //
        // In 4.1, argument pointers were standardized to always be `T**`.
        #[cfg(before_api = "4.1")]
        {
            self.sys()
        }

        #[cfg(since_api = "4.1")]
        {
            ptr::addr_of!(self.obj) as sys::GDExtensionConstTypePtr
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
        opt.unwrap_or_else(|| Self::null())
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
        let is_last = unsafe { T::DynMemory::maybe_dec_ref(self) }; // may drop
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
            let is_last = T::DynMemory::maybe_dec_ref(&self); // may drop
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
            self.check_rtti("clone");
        }

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
            Variant::new_with_var_uninit(|variant_ptr| {
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
            RawGd::<classes::Object>::new_with_uninit(|self_ptr| {
                let converter = sys::builtin_fn!(object_from_variant);
                converter(self_ptr, sys::SysPtr::force_mut(variant.var_sys()));
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
