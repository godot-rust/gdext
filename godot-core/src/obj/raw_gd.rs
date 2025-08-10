/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{fmt, ptr};

use godot_ffi as sys;
use sys::{interface_fn, ExtVariantType, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::{Variant, VariantType};
use crate::meta::error::{ConvertError, FromVariantError};
use crate::meta::{
    CallContext, ClassName, FromGodot, GodotConvert, GodotFfiVariant, GodotType, RefArg, ToGodot,
};
use crate::obj::bounds::{Declarer, DynMemory as _};
use crate::obj::casts::CastSuccess;
use crate::obj::rtti::ObjectRtti;
use crate::obj::{bounds, Bounds, Gd, GdDerefTarget, GdMut, GdRef, GodotClass, InstanceId};
use crate::storage::{InstanceCache, InstanceStorage, Storage};
use crate::{classes, out};

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

    // Direct access to InstanceStorage -- initially null.
    // Only set for user-defined types; ZST otherwise.
    cached_storage_ptr: <<T as Bounds>::Declarer as Declarer>::InstanceCache,
}

impl<T: GodotClass> RawGd<T> {
    /// Initializes this `RawGd<T>` from the object pointer as a **weak ref**, meaning it does not
    /// initialize/increment the reference counter.
    ///
    /// If `obj` is null or the instance ID query behind the object returns 0, the returned `RawGd<T>` will have the null state.
    ///
    /// # Safety
    ///
    /// `obj` must be a valid object pointer or a null pointer.
    pub(super) unsafe fn from_obj_sys_weak(obj: sys::GDExtensionObjectPtr) -> Self {
        let rtti = if obj.is_null() {
            None
        } else {
            let raw_id = unsafe { interface_fn!(object_get_instance_id)(obj) };

            // This happened originally during Variant -> RawGd conversion, but at this point it's too late to detect, and UB has already
            // occurred (the Variant holds the object pointer as bytes in an array, which becomes dangling the moment the actual object dies).
            let instance_id = InstanceId::try_from_u64(raw_id)
                .expect("null instance ID when constructing object; this very likely causes UB");

            // TODO(bromeon): this should query dynamic type of object, which can be different from T (upcast, FromGodot, etc).
            // See comment in ObjectRtti.
            Some(ObjectRtti::of::<T>(instance_id))
        };

        Self {
            obj: obj.cast::<T>(),
            cached_rtti: rtti,
            cached_storage_ptr: InstanceCache::null(),
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
    /// This does not check if the object is dead. For that, use [`is_instance_valid()`](Self::is_instance_valid).
    pub(crate) fn is_null(&self) -> bool {
        self.obj.is_null() || self.cached_rtti.is_none()
    }

    pub(crate) fn instance_id_unchecked(&self) -> Option<InstanceId> {
        self.cached_rtti.as_ref().map(|rtti| rtti.instance_id())
    }

    pub(crate) fn is_instance_valid(&self) -> bool {
        self.cached_rtti
            .as_ref()
            .is_some_and(|rtti| rtti.instance_id().lookup_validity())
    }

    // See use-site for explanation.
    fn is_cast_valid<U>(&self) -> bool
    where
        U: GodotClass,
    {
        self.is_null() // Null can be cast to anything.
            || self.as_object_ref().is_class(&U::class_name().to_gstring())
    }

    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error
    pub(super) fn owned_cast<U>(self) -> Result<RawGd<U>, Self>
    where
        U: GodotClass,
    {
        // Workaround for bug in Godot that makes casts always succeed (https://github.com/godot-rust/gdext/issues/158).
        // TODO once fixed in Godot, remove this.
        if !self.is_cast_valid::<U>() {
            return Err(self);
        }

        // The unsafe { std::mem::transmute<&T, &Base>(self.inner()) } relies on the C++ static_cast class casts
        // to return the same pointer, however in theory those may yield a different pointer (VTable offset,
        // virtual inheritance etc.). It *seems* to work so far, but this is no indication it's not UB.
        //
        // The Deref/DerefMut impls for T implement an "implicit upcast" on the object (not Gd) level and
        // rely on this (e.g. &Node3D -> &Node).

        match self.ffi_cast::<U>() {
            Ok(success) => Ok(success.into_dest(self)),
            Err(_) => Err(self),
        }
    }

    /// Low-level cast that allows selective use of either input or output type.
    ///
    /// On success, you'll get a `CastSuccess<T, U>` instance, which holds a weak `RawGd<U>`. You can only extract that one by trading
    /// a strong `RawGd<T>` for it, to maintain the balance.
    ///
    /// This function is unreliable when invoked _during_ destruction (e.g. C++ `~RefCounted()` destructor). This can occur when debug-logging
    /// instances during cleanups. `Object::object_cast_to()` is a virtual function, but virtual dispatch during destructor doesn't work in C++.
    pub(super) fn ffi_cast<U>(&self) -> Result<CastSuccess<T, U>, ()>
    where
        U: GodotClass,
    {
        //eprintln!("ffi_cast: {} (dyn {}) -> {}", T::class_name(), self.as_non_null().dynamic_class_string(), U::class_name());

        // `self` may be null when we convert a null-variant into a `Option<Gd<T>>`, since we use `ffi_cast`
        // in the `ffi_from_variant` conversion function to ensure type-correctness. So the chain would be as follows:
        // - Variant::nil()
        // - null RawGd<Object>
        // - null RawGd<T>
        // - Option::<Gd<T>>::None
        if self.is_null() {
            // Null can be cast to anything.
            // Forgetting a null doesn't do anything, since dropping a null also does nothing.
            return Ok(CastSuccess::null());
        }

        // Before Godot API calls, make sure the object is alive (and in Debug mode, of the correct type).
        // Current design decision: EVERY cast fails on incorrect type, even if target type is correct. This avoids the risk of violated
        // invariants that leak to the Godot implementation. Also, we do not provide a way to recover from bad types -- this is always
        // a bug that must be solved by the user.
        self.check_rtti("ffi_cast");

        let cast_object_ptr = unsafe {
            let class_tag = interface_fn!(classdb_get_class_tag)(U::class_name().string_sys());
            interface_fn!(object_cast_to)(self.obj_sys(), class_tag)
        };

        if cast_object_ptr.is_null() {
            return Err(());
        }

        // Create weak object, as ownership will be moved and reference-counter stays the same.
        let weak = unsafe { RawGd::from_obj_sys_weak(cast_object_ptr) };
        Ok(CastSuccess::from_weak(weak))
    }

    /// Executes a function, assuming that `self` inherits `RefCounted`.
    ///
    /// This function is unreliable when invoked _during_ destruction (e.g. C++ `~RefCounted()` destructor). This can occur when debug-logging
    /// instances during cleanups. `Object::object_cast_to()` is a virtual function, but virtual dispatch during destructor doesn't work in C++.
    ///
    /// # Panics
    /// If `self` does not inherit `RefCounted` or is null.
    pub fn with_ref_counted<R>(&self, apply: impl Fn(&mut classes::RefCounted) -> R) -> R {
        // Note: this previously called Declarer::scoped_mut() - however, no need to go through bind() for changes in base RefCounted.
        // Any accesses to user objects (e.g. destruction if refc=0) would bind anyway.
        //
        // Might change implementation as follows -- but last time caused UB; investigate.
        // pub(crate) unsafe fn as_ref_counted_unchecked(&mut self) -> &mut classes::RefCounted {
        //     self.as_target_mut()
        // }

        match self.try_with_ref_counted(apply) {
            Ok(result) => result,
            Err(()) if self.is_null() => {
                panic!("RawGd::with_ref_counted(): expected to inherit RefCounted, encountered null pointer");
            }
            Err(()) => {
                // SAFETY: this branch implies non-null.
                let gd_ref = unsafe { self.as_non_null() };
                let class = gd_ref.dynamic_class_string();

                // One way how this may panic is when invoked during destruction of a RefCounted object. The C++ `Object::object_cast_to()`
                // function is virtual but cannot be dynamically dispatched in a C++ destructor.
                panic!(
                    "Operation not permitted for object of class {class}:\n\
                    class is either not RefCounted, or currently in construction/destruction phase"
                );
            }
        }
    }

    /// Fallible version of [`with_ref_counted()`](Self::with_ref_counted), for situations during init/drop when downcast no longer works.
    #[expect(clippy::result_unit_err)]
    pub fn try_with_ref_counted<R>(
        &self,
        apply: impl Fn(&mut classes::RefCounted) -> R,
    ) -> Result<R, ()> {
        let mut ref_counted = self.ffi_cast::<classes::RefCounted>()?;
        let return_val = apply(ref_counted.as_dest_mut().as_target_mut());

        // CastSuccess is forgotten when dropped, so no ownership transfer.
        Ok(return_val)
    }

    /// Enables outer `Gd` APIs or bypasses additional null checks, in cases where `RawGd` is guaranteed non-null.
    ///
    /// # Safety
    /// `self` must not be null.
    pub(crate) unsafe fn as_non_null(&self) -> &Gd<T> {
        debug_assert!(
            !self.is_null(),
            "RawGd::as_non_null() called on null pointer; this is UB"
        );

        // SAFETY: layout of Gd<T> is currently equivalent to RawGd<T>.
        unsafe { std::mem::transmute::<&RawGd<T>, &Gd<T>>(self) }
    }

    pub(crate) fn as_object_ref(&self) -> &classes::Object {
        // SAFETY: Object is always a valid upcast target.
        unsafe { self.as_upcast_ref() }
    }

    pub(crate) fn as_object_mut(&mut self) -> &mut classes::Object {
        // SAFETY: Object is always a valid upcast target.
        unsafe { self.as_upcast_mut() }
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
        // DeclEngine needed for sound transmute; in case we add Rust-defined base classes.
        Base: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
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
        //     cached_storage_ptr: InstanceCache, // ZST for engine classes.
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
        // DeclEngine needed for sound transmute; in case we add Rust-defined base classes.
        Base: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
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
    pub(super) fn as_target(&self) -> &GdDerefTarget<T>
    where
        GdDerefTarget<T>: Bounds<Declarer = bounds::DeclEngine>,
    {
        // SAFETY: There are two possible Declarer::DerefTarget types:
        // - T, if T is an engine class
        // - T::Base, if T is a user class
        // Both are valid targets for upcast. And both are always engine types.
        unsafe { self.as_upcast_ref::<GdDerefTarget<T>>() }
    }

    /// # Panics
    /// If this `RawGd` is null.
    pub(super) fn as_target_mut(&mut self) -> &mut GdDerefTarget<T>
    where
        GdDerefTarget<T>: Bounds<Declarer = bounds::DeclEngine>,
    {
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
            let ffi_dest = self.ffi_cast::<Base>().expect("failed FFI upcast");

            // The ID check is not that expressive; we should do a complete comparison of the ObjectRtti, but currently the dynamic types can
            // be different (see comment in ObjectRtti struct). This at least checks that the transmuted object is not complete garbage.
            // We get direct_id from Self and not Base because the latter has no API with current bounds; but this equivalence is tested in Deref.
            let direct_id = self.instance_id_unchecked().expect("direct_id null");
            let ffi_id = ffi_dest
                .as_dest_ref()
                .instance_id_unchecked()
                .expect("ffi_id null");

            assert_eq!(
                direct_id, ffi_id,
                "upcast_ref: direct and FFI IDs differ. This is a bug, please report to godot-rust maintainers."
            );
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

    // Not pub(super) because used by godot::meta::args::ObjectArg.
    pub(crate) fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
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
    pub(crate) fn bind(&self) -> GdRef<'_, T> {
        self.check_rtti("bind");
        GdRef::from_guard(self.storage().unwrap().get())
    }

    /// Hands out a guard for an exclusive borrow, through which the user instance can be read and written.
    ///
    /// See [`crate::obj::Gd::bind_mut()`] for a more in depth explanation.
    pub(crate) fn bind_mut(&mut self) -> GdMut<'_, T> {
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

    /// Retrieves and caches pointer to this class instance if `self.obj` is non-null.
    /// Returns a null pointer otherwise.
    ///
    /// Note: The returned pointer to the GDExtensionClass instance (even when `self.obj` is non-null)
    /// might still be null when:
    /// - The class isn't instantiable in the current context.
    /// - The instance is a placeholder (e.g., non-`tool` classes in the editor).
    ///
    /// However, null pointers might also occur in other, undocumented contexts.
    ///
    /// # Panics
    /// In Debug mode, if binding is null.
    fn resolve_instance_ptr(&self) -> sys::GDExtensionClassInstancePtr {
        if self.is_null() {
            return ptr::null_mut();
        }

        let cached = self.cached_storage_ptr.get();
        if !cached.is_null() {
            return cached;
        }

        let callbacks = crate::storage::nop_instance_callbacks();

        // SAFETY: library is already initialized.
        let token = unsafe { sys::get_library() };
        let token = token.cast::<std::ffi::c_void>();

        // SAFETY: ensured that `self.obj` is non-null and valid.
        let binding = unsafe {
            interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks)
        };

        let ptr: sys::GDExtensionClassInstancePtr = binding.cast();

        #[cfg(debug_assertions)]
        crate::classes::ensure_binding_not_null::<T>(ptr);

        self.cached_storage_ptr.set(ptr);
        ptr
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
    // If anything changes here, keep in sync with ObjectArg impl.

    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::OBJECT);

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

    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        // Even though ObjectArg exists, this function is still relevant, e.g. in Callable.

        object_as_arg_ptr(&self.obj)
    }

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self {
        if ptr.is_null() {
            return Self::null();
        }

        let obj_ptr = if T::DynMemory::pass_as_ref(call_type) {
            // ptr is `Ref<T>*`
            // See the docs for `PtrcallType::Virtual` for more info on `Ref<T>`.
            interface_fn!(ref_get_object)(ptr as sys::GDExtensionRefPtr)
        } else {
            // ptr is `T**` from Godot 4.1 onwards, also in virtual functions.
            *(ptr as *mut sys::GDExtensionObjectPtr)
        };

        // obj_ptr is `T*`
        Self::from_obj_sys(obj_ptr)
    }

    unsafe fn move_return_ptr(self, ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) {
        if T::DynMemory::pass_as_ref(call_type) {
            // ref_set_object creates a new Ref<T> in the engine and increments the reference count. We have to drop our Gd<T> to decrement
            // the reference count again.
            interface_fn!(ref_set_object)(ptr as sys::GDExtensionRefPtr, self.obj_sys());
        } else {
            ptr::write(ptr as *mut _, self.obj);
            // We've passed ownership to caller.
            std::mem::forget(self);
        }
    }
}

impl<T: GodotClass> GodotConvert for RawGd<T> {
    type Via = Self;
}

impl<T: GodotClass> ToGodot for RawGd<T> {
    type ToVia<'v> = Self;

    fn to_godot(&self) -> Self::ToVia<'_> {
        self.clone()
    }
}

impl<T: GodotClass> FromGodot for RawGd<T> {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }
}

impl<T: GodotClass> GodotType for RawGd<T> {
    type Ffi = Self;

    type ToFfi<'f>
        = RefArg<'f, RawGd<T>>
    where
        Self: 'f;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        RefArg::new(self)
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
        object_ffi_to_variant(self)
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        let variant_type = variant.get_type();

        // Explicit type check before calling `object_from_variant`, to allow for better error messages.
        if variant_type != VariantType::OBJECT {
            return Err(FromVariantError::BadType {
                expected: VariantType::OBJECT,
                actual: variant_type,
            }
            .into_error(variant.clone()));
        }

        // Check for dead objects *before* converting. Godot doesn't care if the objects are still alive, and hitting
        // RawGd::from_obj_sys_weak() is too late and causes UB.
        if !variant.is_object_alive() {
            return Err(FromVariantError::DeadObject.into_error(variant.clone()));
        }

        let raw = unsafe {
            // Uses RawGd<Object> and not Self, because Godot still allows illegal conversions. We thus check with manual casting later on.
            // See https://github.com/godot-rust/gdext/issues/158.

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

impl<T: GodotClass> GodotNullableFfi for RawGd<T> {
    /// Create a new object representing a null in Godot.
    fn null() -> Self {
        Self {
            obj: ptr::null_mut(),
            cached_rtti: None,
            cached_storage_ptr: InstanceCache::null(),
        }
    }

    fn is_null(&self) -> bool {
        Self::is_null(self)
    }
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

        out!("RawGd::drop:      {self:?}");

        // SAFETY: This `Gd` won't be dropped again after this.
        // If destruction is triggered by Godot, Storage already knows about it, no need to notify it
        let is_last = unsafe { T::DynMemory::maybe_dec_ref(self) }; // may drop
        if is_last {
            unsafe {
                interface_fn!(object_destroy)(self.obj_sys());
            }
        }
    }
}

impl<T: GodotClass> Clone for RawGd<T> {
    fn clone(&self) -> Self {
        out!("RawGd::clone:     {self:?}  (before clone)");

        let cloned = if self.is_null() {
            Self::null()
        } else {
            self.check_rtti("clone");

            // Create new object, adopt cached fields.
            let copy = Self {
                obj: self.obj,
                cached_rtti: self.cached_rtti.clone(),
                cached_storage_ptr: self.cached_storage_ptr.clone(),
            };
            copy.with_inc_refcount()
        };

        out!("                  {self:?}  (after clone)");
        cloned
    }
}

impl<T: GodotClass> fmt::Debug for RawGd<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        classes::debug_string_nullable(self, f, "RawGd")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Reusable functions, also shared with Gd, Variant, ObjectArg.

/// Runs `init_fn` on the address of a pointer (initialized to null), then returns that pointer, possibly still null.
///
/// This relies on the fact that an object pointer takes up the same space as the FFI representation of an object (`OpaqueObject`).
/// The pointer is thus used as an opaque handle, initialized by `init_fn`, so that it represents a valid Godot object afterwards.
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

// Used by godot::meta::args::ObjectArg + local impl GodotFfiVariant.
pub(crate) fn object_ffi_to_variant<T: GodotFfi>(self_: &T) -> Variant {
    // The conversion method `object_to_variant` DOES increment the reference-count of the object; so nothing to do here.
    // (This behaves differently in the opposite direction `variant_to_object`.)

    unsafe {
        Variant::new_with_var_uninit(|variant_ptr| {
            let converter = sys::builtin_fn!(object_to_variant);

            // Note: this is a special case because of an inconsistency in Godot, where sometimes the equivalency is
            // GDExtensionTypePtr == Object** and sometimes GDExtensionTypePtr == Object*. Here, it is the former, thus extra pointer.
            // Reported at https://github.com/godotengine/godot/issues/61967
            let type_ptr = self_.sys();
            converter(
                variant_ptr,
                ptr::addr_of!(type_ptr) as sys::GDExtensionTypePtr,
            );
        })
    }
}

// Used by godot::meta::args::ObjectArg.
pub(crate) fn object_as_arg_ptr<F>(_object_ptr_field: &*mut F) -> sys::GDExtensionConstTypePtr {
    // Be careful when refactoring this code. Address of field pointer matters, copying it into a local variable will create use-after-free.

    // No need to call self.check_rtti("as_arg_ptr") here, since this is already done in ToGodot impl.

    // We pass an object to a Godot API. If the reference count needs to be incremented, then the callee (Godot C++ function) will do so.
    // We do not need to prematurely do so. In Rust terms, if `T` is ref-counted, then we are effectively passing a `&Arc<T>`, and the
    // callee would need to invoke `.clone()` if desired.

    // Since 4.1, argument pointers were standardized to always be `T**`.
    ptr::addr_of!(*_object_ptr_field) as sys::GDExtensionConstTypePtr
}
