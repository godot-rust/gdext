/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

use godot_ffi as sys;
use godot_ffi::VariantType;
use sys::{interface_fn, GodotFfi, PtrcallType};

use crate::builtin::meta::{ClassName, VariantMetadata};
use crate::builtin::{FromVariant, GodotString, ToVariant, Variant, VariantConversionError};
use crate::engine::{Node, Object, Resource};
use crate::obj::dom::Domain as _;
use crate::obj::mem::Memory as _;
use crate::obj::{cap, dom, EngineEnum, GodotClass};
use crate::obj::{GdMut, GdRef, InstanceId};
use crate::property::{Export, ExportInfo, Property, TypeStringHint};
use crate::storage::InstanceStorage;
use crate::{callbacks, engine};

/// A raw pointer to an instance of a class.
///
/// This is not intended for general use, you usually want to use [`Gd`](super::Gd) instead of using this
/// directly.
///
/// `RawGd` is not guaranteed to be live, nor non-null. But many of the methods are only safe to call on null
/// or live instances.
pub struct RawGd<T: GodotClass> {
    // Store a pointer, as transmuting between pointers and integers can be problematic:
    // https://github.com/rust-lang/unsafe-code-guidelines/issues/286
    ptr: sys::GDExtensionObjectPtr,
    // `RawGd` should be usable in all contexts, `Cell` would make it very difficult to use it in
    // multithreaded contexts.
    cached_instance_id: AtomicU64,
    _marker: PhantomData<*const T>,
}

impl<T: GodotClass> RawGd<T> {
    /// # Safety
    /// `ptr` must not be a pointer to a freed non-null object.
    #[doc(hidden)]
    pub(crate) unsafe fn from_obj_sys(ptr: sys::GDExtensionObjectPtr) -> Self {
        let mut obj = Self {
            ptr,
            cached_instance_id: AtomicU64::new(0),
            _marker: PhantomData,
        };

        if obj.is_null() {
            return obj;
        }

        // SAFETY: `obj` does not point to a freed or null object.
        let id = unsafe { interface_fn!(object_get_instance_id)(obj.obj_sys()) };
        obj.cached_instance_id = AtomicU64::new(id);

        obj
    }

    /// Construct a new null object. This does not require any ffi.
    pub fn new_null() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            cached_instance_id: AtomicU64::new(0),
            _marker: PhantomData,
        }
    }

    /// Looks up the given instance ID and returns the associated object, if possible.
    ///
    /// If no such instance ID is registered, or if the dynamic type of the object behind that instance ID
    /// is not compatible with `T`, then `None` is returned.
    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        // SAFETY: Godot looks up ID in ObjectDB and returns null if not found
        let ptr = unsafe { interface_fn!(object_get_instance_from_id)(instance_id.to_u64()) };

        // SAFETY: assumes that the returned GDExtensionObjectPtr is convertible to Object* (i.e. C++ upcast doesn't modify the pointer)
        let untyped = unsafe { RawGd::<engine::Object>::from_obj_sys(ptr) };
        if untyped.is_instance_valid() {
            untyped.owned_cast::<T>().ok()
        } else {
            None
        }
    }

    fn set_instance_id(&self, id: Option<InstanceId>) {
        let id = InstanceId::option_to_u64(id);
        self.cached_instance_id.store(id, Ordering::Release);
    }

    /// Return the cached instance id without checking if the object is still live or not.
    pub fn instance_id_or_none_unchecked(&self) -> Option<InstanceId> {
        let id = self.cached_instance_id.load(Ordering::Acquire);
        InstanceId::try_from_u64(id)
    }

    /// Return the instance id of this instance, or `None` if the object is null or has been freed.
    pub fn instance_id_or_none(&self) -> Option<InstanceId> {
        let id = self.instance_id_or_none_unchecked()?;

        // Refreshes the internal cached ID on every call, as we cannot be sure that the object has not been
        // destroyed since last time. The only reliable way to find out is to call is_instance_id_valid().
        if engine::utilities::is_instance_id_valid(id.to_i64()) {
            Some(id)
        } else {
            self.set_instance_id(None);
            None
        }
    }

    /// Checks if this pointer points to a live object.
    ///
    /// Many `unsafe` methods of `RawGd` require the object to be live before they can be safely called, this
    /// is the main way to check that.
    pub fn is_instance_valid(&self) -> bool {
        self.instance_id_or_none().is_some()
    }

    /// Checks if this is a null-pointer, this does not check if the pointer points to a live object, for
    /// that you should use [`is_instance_valid()`].
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Cast an object directly using `object_cast_to`.
    pub(super) fn ffi_cast<U>(&self) -> Option<RawGd<U>>
    where
        U: GodotClass,
    {
        if !self.is_instance_valid() {
            return None;
        }

        unsafe {
            let class_name = ClassName::of::<U>();
            let class_tag = interface_fn!(classdb_get_class_tag)(class_name.string_sys());
            let cast_object_ptr = interface_fn!(object_cast_to)(self.obj_sys(), class_tag);

            sys::ptr_then(cast_object_ptr, |ptr| RawGd::from_obj_sys(ptr))
        }
    }

    // Temporary workaround for bug in Godot that makes casts always succeed.
    // (See https://github.com/godot-rust/gdext/issues/158)
    // TODO(#234) remove this code once the bug is fixed upstream.
    fn is_cast_valid<U>(&self) -> bool
    where
        U: GodotClass,
    {
        let as_obj = self
            .ffi_cast::<Object>()
            .expect("Everything inherits object");
        let cast_is_valid = unsafe { as_obj.as_inner() }.is_class(GodotString::from(U::CLASS_NAME));
        cast_is_valid
    }

    /// Take ownership of `self` and try to cast it to another type.
    ///
    /// Returns `Ok(cast_obj)` on success, `Err(self)` on error
    pub(super) fn owned_cast<U>(self) -> Result<RawGd<U>, Self>
    where
        U: GodotClass,
    {
        // Temporary workaround for bug in Godot that makes casts always
        // succeed. (See https://github.com/godot-rust/gdext/issues/158)
        // TODO(#234) remove this check once the bug is fixed upstream.
        if !self.is_cast_valid::<U>() {
            return Err(self);
        }

        // The unsafe { std::mem::transmute<&T, &Base>(self.inner()) } relies on the C++ static_cast class casts
        // to return the same pointer, however in theory those may yield a different pointer (VTable offset,
        // virtual inheritance etc.). It *seems* to work so far, but this is no indication it's not UB.
        //
        // The Deref/DerefMut impls for T implement an "implicit upcast" on the object (not Gd) level and
        // rely on this (e.g. &Node3D -> &Node).

        let result = self.ffi_cast::<U>();
        match result {
            Some(cast_obj) => Ok(cast_obj),
            None => Err(self),
        }
    }

    /// Call the function `apply` on `self`, assuming that `self` is a [`RefCounted`](crate::engine::RefCounted) object.
    pub(crate) fn as_ref_counted<R>(&self, apply: impl Fn(&mut engine::RefCounted) -> R) -> R {
        debug_assert!(
            self.is_instance_valid(),
            "as_ref_counted() on freed instance; maybe forgot to increment reference count?"
        );

        let tmp = self.ffi_cast::<engine::RefCounted>();
        let mut tmp = tmp.expect("object expected to inherit RefCounted");
        // SAFETY: instance is valid.
        unsafe {
            <engine::RefCounted as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj))
        }
    }

    /// Call the function `apply` on `self` casted to an `Object`.
    ///
    /// # Safety
    ///
    /// `self` must be a valid instance, or `apply` must only call methods that can be called on freed/null
    /// objects.
    pub(crate) unsafe fn as_object<R>(&self, apply: impl Fn(&mut engine::Object) -> R) -> R {
        // Note: no validity check; this could be called by to_string(), which can be called on dead instances

        let tmp = self.ffi_cast::<engine::Object>();
        let mut tmp = tmp.expect("object expected to inherit Object; should never fail");
        <engine::Object as GodotClass>::Declarer::scoped_mut(&mut tmp, |obj| apply(obj))
    }

    /// Returns `self` but with initialized ref-count.
    pub(super) fn with_inc_refcount(self) -> Self {
        // Note: use init_ref and not inc_ref, since this might be the first reference increment.
        // Godot expects RefCounted::init_ref to be called instead of RefCounted::reference in that case.
        // init_ref also doesn't hurt (except 1 possibly unnecessary check).
        T::Mem::maybe_init_ref(&self);
        self
    }

    /// Format `self` for debug printing. Using the `ty` string for the name of the smart pointer.
    pub(crate) fn debug_string(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        ty: &str,
    ) -> std::fmt::Result {
        if let Some(id) = self.instance_id_or_none() {
            // SAFETY: `self` isn't freed.
            let class: GodotString = unsafe { self.as_object(|obj| Object::get_class(obj)) };

            write!(f, "{ty} {{ id: {id}, class: {class} }}")
        } else {
            write!(f, "{ty} {{ freed obj }}")
        }
    }

    /// Format `self` for display printing.
    pub(crate) fn display_string(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: May be called on freed instances.
        let string: GodotString = unsafe { self.as_object(Object::to_string) };

        <GodotString as std::fmt::Display>::fmt(&string, f)
    }

    /// # Safety
    /// - `self` must not be a dead pointer.
    /// - `self` must not be used again after this, this includes dropping a [`Gd`](super::Gd) which inherits
    /// from [`RefCounted`](crate::engine::RefCounted) .
    pub unsafe fn free(self) {
        // This destroys the Storage instance, no need to run destructor again
        unsafe {
            interface_fn!(object_destroy)(self.obj_sys());
        }
    }

    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.ptr
    }
}

impl<T> RawGd<T>
where
    T: GodotClass<Declarer = dom::UserDomain>,
{
    /// Moves a user-created object into this smart pointer, submitting ownership to the Godot engine.
    ///
    /// This is only useful for types `T` which do not store their base objects (if they have a base,
    /// you cannot construct them standalone).
    pub fn new(user_object: T) -> Self {
        Self::with_base(move |_base| user_object)
    }

    /// Creates a default-constructed instance of `T` inside a smart pointer.
    ///
    /// This is equivalent to the GDScript expression `T.new()`.
    pub fn new_default() -> Self
    where
        T: cap::GodotInit,
    {
        unsafe {
            let object_ptr = callbacks::create::<T>(ptr::null_mut());
            RawGd::from_obj_sys(object_ptr)
        }
    }

    /// Creates a `RawGd<T>` using a function that constructs a `T` from a provided base.
    ///
    /// See also [`Gd::with_base`].
    pub fn with_base<F>(init: F) -> Self
    where
        F: FnOnce(crate::obj::Base<T::Base>) -> T,
    {
        let object_ptr = callbacks::create_custom(init);
        unsafe { RawGd::from_obj_sys(object_ptr) }
    }

    /// Hands out a guard for a shared borrow, through which the user instance can be read.
    ///
    /// The pattern is very similar to interior mutability with standard [`RefCell`][std::cell::RefCell].
    /// You can either have multiple `GdRef` shared guards, or a single `GdMut` exclusive guard to a Rust
    /// `GodotClass` instance, independently of how many smart pointers point to it. There are runtime
    /// checks to ensure that Rust safety rules (e.g. no `&` and `&mut` coexistence) are upheld.
    ///
    /// # Panics
    /// * If another smart pointer pointing to the same Rust instance has a live `GdMut` guard bound.
    /// * If there is an ongoing function call from GDScript to Rust, which currently holds a `&mut T`
    ///   reference to the user instance. This can happen through re-entrancy (Rust -> GDScript -> Rust call).
    // Note: possible names: write/read, hold/hold_mut, r/w, r/rw, ...
    pub fn bind(&self) -> GdRef<T> {
        GdRef::from_cell(self.storage().get())
    }

    /// Hands out a guard for an exclusive borrow, through which the user instance can be read and written.
    ///
    /// The pattern is very similar to interior mutability with standard [`RefCell`][std::cell::RefCell].
    /// You can either have multiple `GdRef` shared guards, or a single `GdMut` exclusive guard to a Rust
    /// `GodotClass` instance, independently of how many smart pointers point to it. There are runtime
    /// checks to ensure that Rust safety rules (e.g. no `&mut` aliasing) are upheld.
    ///
    /// # Panics
    /// * If another smart pointer pointing to the same Rust instance has a live `GdRef` or `GdMut` guard bound.
    /// * If there is an ongoing function call from GDScript to Rust, which currently holds a `&T` or `&mut T`
    ///   reference to the user instance. This can happen through re-entrancy (Rust -> GDScript -> Rust call).
    pub fn bind_mut(&mut self) -> GdMut<T> {
        GdMut::from_cell(self.storage().get_mut())
    }

    /// Storage object associated with the extension instance.
    pub(crate) fn storage(&self) -> &InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library() as *mut std::ffi::c_void;
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);

            debug_assert!(
                !binding.is_null(),
                "Class {} -- null instance; does the class have a Godot creator function?",
                std::any::type_name::<T>()
            );
            crate::private::as_storage::<T>(binding as sys::GDExtensionClassInstancePtr)
        }
    }
}

impl<T> RawGd<T>
where
    T: GodotClass<Declarer = dom::EngineDomain>,
{
    /// Get a reference to the underlying Godot object.
    ///
    /// # Safety
    ///
    /// `self` must not have been previously freed.
    pub unsafe fn as_inner(&self) -> &T {
        let addr = std::ptr::addr_of!(self.ptr);
        &*(addr as *const T)
    }

    /// Get a mutable reference to the underlying Godot object.
    ///
    /// # Safety
    ///
    /// `self` must not have been previously freed.
    pub unsafe fn as_inner_mut(&mut self) -> &mut T {
        let addr = std::ptr::addr_of_mut!(self.ptr);
        &mut *(addr as *mut T)
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
    unsafe fn from_sys(ptr: sys::GDExtensionTypePtr) -> Self {
        Self::from_obj_sys(ptr as sys::GDExtensionObjectPtr)
    }

    unsafe fn from_sys_init(
        init: impl FnOnce(<sys::GDExtensionTypePtr as sys::AsUninit>::Ptr),
    ) -> Self {
        let mut raw: MaybeUninit<sys::GDExtensionObjectPtr> = std::mem::MaybeUninit::uninit();
        init(raw.as_mut_ptr() as sys::GDExtensionUninitializedTypePtr);
        Self::from_obj_sys(raw.assume_init())
    }

    fn sys(&self) -> sys::GDExtensionTypePtr {
        self.ptr as sys::GDExtensionTypePtr
    }

    // For more context around `ref_get_object` and `ref_set_object`, see:
    // https://github.com/godotengine/godot-cpp/issues/954

    unsafe fn from_arg_ptr(ptr: sys::GDExtensionTypePtr, call_type: PtrcallType) -> Self {
        if ptr.is_null() {
            return Self::new_null();
        }

        let obj_ptr = if T::Mem::pass_as_ref(call_type) {
            // ptr is `Ref<T>*`
            // See the docs for `PtrcallType::Virtual` for more info on `Ref<T>`.
            interface_fn!(ref_get_object)(ptr as sys::GDExtensionRefPtr)
        } else if !cfg!(gdextension_api = "4.0") || matches!(call_type, PtrcallType::Virtual) {
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
            std::ptr::write(ptr as *mut _, self.ptr)
        }
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
        if cfg!(gdextension_api = "4.0") {
            self.sys_const()
        } else {
            std::ptr::addr_of!(self.ptr) as sys::GDExtensionConstTypePtr
        }
    }
}

impl<T: GodotClass> Clone for RawGd<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            // It's not important that we grab the latest instance id, just that it is either the
            // correct instance id for the opaque object, or 0. So we can use `Relaxed`.
            cached_instance_id: self.cached_instance_id.load(Ordering::Relaxed).into(),
            _marker: self._marker,
        }
    }
}

impl<T: GodotClass> TypeStringHint for RawGd<T> {
    fn type_string() -> String {
        use engine::global::PropertyHint;

        match Self::default_export_info().hint {
            hint @ (PropertyHint::PROPERTY_HINT_RESOURCE_TYPE
            | PropertyHint::PROPERTY_HINT_NODE_TYPE) => {
                format!(
                    "{}/{}:{}",
                    VariantType::Object as i32,
                    hint.ord(),
                    T::CLASS_NAME
                )
            }
            _ => format!("{}:", VariantType::Object as i32),
        }
    }
}

impl<T: GodotClass> Property for RawGd<T> {
    type Intermediate = Self;

    fn get_property(&self) -> Self {
        self.clone()
    }

    fn set_property(&mut self, value: Self) {
        *self = value;
    }
}

impl<T: GodotClass> Export for RawGd<T> {
    fn default_export_info() -> ExportInfo {
        let hint = if T::inherits::<Resource>() {
            engine::global::PropertyHint::PROPERTY_HINT_RESOURCE_TYPE
        } else if T::inherits::<Node>() {
            engine::global::PropertyHint::PROPERTY_HINT_NODE_TYPE
        } else {
            engine::global::PropertyHint::PROPERTY_HINT_NONE
        };

        // Godot does this by default too, it doesn't seem to make a difference when not a resource/node
        // but is needed when it is a resource/node.
        let hint_string = T::CLASS_NAME.into();

        ExportInfo { hint, hint_string }
    }
}

impl<T: GodotClass> FromVariant for RawGd<T> {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        if variant.is_nil() {
            return Ok(Self::new_null());
        }

        let raw = unsafe {
            // TODO(#234) replace Gd::<Object> with Self when Godot stops allowing illegal conversions
            // See https://github.com/godot-rust/gdext/issues/158

            RawGd::<Object>::from_sys_init(|self_ptr| {
                let converter = sys::builtin_fn!(object_from_variant);
                converter(self_ptr, variant.var_sys());
            })
        };

        // TODO(#234) remove this cast when Godot stops allowing illegal conversions
        // (See https://github.com/godot-rust/gdext/issues/158)
        raw.owned_cast()
            .map_err(|_| VariantConversionError::BadType)
    }
}

impl<T: GodotClass> ToVariant for RawGd<T> {
    fn to_variant(&self) -> Variant {
        // This method increments the refcount, which is fine since it'll automatically be decremented when
        // the `Variant` is dropped. This does mean however that creating a new `RawGd` and then turning it
        // into a `Variant` will cause the `RawGd` to be freed, since its refcount will be 0.

        unsafe {
            Variant::from_var_sys_init(|variant_ptr| {
                let converter = sys::builtin_fn!(object_to_variant);

                // Note: this is a special case because of an inconsistency in Godot 4.0, where sometimes the
                // equivalency is GDExtensionTypePtr == Object** and sometimes GDExtensionTypePtr == Object*.
                // Here, it is the former, thus extra pointer.
                let type_ptr = self.sys();
                converter(
                    variant_ptr,
                    ptr::addr_of!(type_ptr) as sys::GDExtensionTypePtr,
                );
            })
        }
    }
}

impl<T: GodotClass> PartialEq for RawGd<T> {
    /// Returns whether two `Gd` pointers point to the same object, or are both null/freed.
    fn eq(&self, other: &Self) -> bool {
        self.instance_id_or_none() == other.instance_id_or_none()
    }
}

impl<T: GodotClass> Eq for RawGd<T> {}

impl<T: GodotClass> Display for RawGd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.display_string(f)
    }
}

impl<T: GodotClass> Debug for RawGd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.debug_string(f, "RawGd")
    }
}

impl<T: GodotClass> VariantMetadata for RawGd<T> {
    fn variant_type() -> VariantType {
        VariantType::Object
    }

    fn class_name() -> ClassName {
        ClassName::of::<T>()
    }
}

// Gd unwinding across panics does not invalidate any invariants;
// its mutability is anyway present, in the Godot engine.
impl<T: GodotClass> std::panic::UnwindSafe for RawGd<T> {}
impl<T: GodotClass> std::panic::RefUnwindSafe for RawGd<T> {}

impl<T: GodotClass> Default for RawGd<T> {
    fn default() -> Self {
        Self::new_null()
    }
}
