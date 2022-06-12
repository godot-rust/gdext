use std::marker::PhantomData;
use std::ptr;

use gdext_builtin::Variant;
use gdext_sys as sys;
use sys::types::OpaqueObject;
use sys::{impl_ffi_as_opaque_value, interface_fn, static_assert_eq_size, GodotFfi};

use crate::property_info::PropertyInfoBuilder;
use crate::storage::InstanceStorage;
use crate::{ClassName, DefaultConstructible, GodotClass, InstanceId};

// TODO which bounds to add on struct itself?
#[repr(transparent)] // needed for safe transmute between object and a field, see EngineClass
pub struct Obj<T: GodotClass> {
    // Note: `opaque` has the same layout as GDNativeObjectPtr == Object* in C++, i.e. the bytes represent a pointer
    // To receive a GDNativeTypePtr == GDNativeObjectPtr* == Object**, we need to get the address of this
    // Hence separate sys() for GDNativeTypePtr, and obj_sys() for GDNativeObjectPtr.
    // The former is the standard FFI type, while the latter is used in object-specific GDExtension APIs.
    opaque: OpaqueObject,
    _marker: PhantomData<*const T>,
}

// Size equality check (should additionally be covered by mem::transmute())
static_assert_eq_size!(
    sys::GDNativeObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

impl<T: GodotClass + DefaultConstructible> Obj<T> {
    pub fn new_default() -> Self {
        let class_name = ClassName::new::<T>();
        let result = unsafe {
            let ptr = interface_fn!(classdb_construct_object)(class_name.c_str());
            Obj::from_obj_sys(ptr)
        };

        result.storage().initialize_default();
        result
    }
}

impl<T: GodotClass> Obj<T> {
    pub fn new(user_object: T) -> Self {
        let class_name = ClassName::new::<T>();
        let result = unsafe {
            let ptr = interface_fn!(classdb_construct_object)(class_name.c_str());
            Obj::from_obj_sys(ptr)
        };

        result.storage().initialize(user_object);
        result
    }

    pub fn try_from_instance_id(instance_id: InstanceId) -> Option<Self> {
        unsafe {
            let ptr = interface_fn!(object_get_instance_from_id)(instance_id.to_u64());

            if ptr.is_null() {
                None
            } else {
                Some(Obj::from_obj_sys(ptr))
            }
        }
    }

    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self::try_from_instance_id(instance_id).expect(&format!(
            "Instance ID {} does not belong to a valid object of class '{}'",
            instance_id,
            T::class_name()
        ))
    }

    fn from_opaque(opaque: OpaqueObject) -> Self {
        Self {
            opaque,
            _marker: PhantomData,
        }
    }

    pub fn instance_id(&self) -> InstanceId {
        // Note: bit 'id & (1 << 63)' determines if the instance is ref-counted
        let id = unsafe { interface_fn!(object_get_instance_id)(self.obj_sys()) };
        InstanceId::from_u64(id)
    }

    // explicit deref for testing purposes
    pub fn inner(&self) -> &T {
        use crate::marker::ClassDeclarer as _;
        T::Declarer::extract_from_obj(self)
    }

    pub fn inner_mut(&mut self) -> &mut T {
        use crate::marker::ClassDeclarer as _;
        T::Declarer::extract_from_obj_mut(self)
    }

    pub(crate) fn storage(&self) -> &mut InstanceStorage<T> {
        let callbacks = crate::storage::nop_instance_callbacks();

        unsafe {
            let token = sys::get_library();
            let binding =
                interface_fn!(object_get_instance_binding)(self.obj_sys(), token, &callbacks);
            crate::private::as_storage::<T>(binding)
        }
    }

    // Conversions from/to Godot C++ `Object*` pointers
    impl_ffi_as_opaque_value!(sys::GDNativeObjectPtr; from_obj_sys, from_obj_sys_init, obj_sys, write_obj_sys);
}

/*
// TODO enable once ownership is clear -- see also forget() in ptrcall_write()
impl<T: GodotClass> Drop for Obj<T>{
    fn drop(&mut self) {
        println!("Obj::drop()");
        unsafe { interface_fn!(object_destroy)(self.sys_mut()); }
    }
}
*/

impl<T: GodotClass> GodotFfi for Obj<T> {
    impl_ffi_as_opaque_value!();
}

impl<T: GodotClass> From<&Variant> for Obj<T> {
    fn from(variant: &Variant) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let converter = sys::method_table().object_from_variant;
                converter(self_ptr, variant.var_sys());
            })
        }
    }
}

impl<T: GodotClass> From<Obj<T>> for Variant {
    fn from(obj: Obj<T>) -> Self {
        Variant::from(&obj)
    }
}

impl<T: GodotClass> From<&Obj<T>> for Variant {
    fn from(obj: &Obj<T>) -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                let converter = sys::method_table().object_to_variant;

                // Note: this is a special case because of an inconsistency in Godot, where sometimes the equivalency is
                // GDNativeTypePtr == Object** and sometimes GDNativeTypePtr == Object*. Here, it is the former, thus extra pointer.
                // Reported at https://github.com/godotengine/godot/issues/61967
                let type_ptr = obj.sys();
                converter(variant_ptr, ptr::addr_of!(type_ptr) as *mut _);
            })
        }
    }
}

impl<T: GodotClass> PropertyInfoBuilder for Obj<T> {
    fn variant_type() -> sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_OBJECT
    }

    fn property_info(name: &str) -> sys::GDNativePropertyInfo {
        // Note: filling this information properly is important so that Godot can use ptrcalls instead of varcalls
        // (requires typed GDScript + sufficient information from the extension side)
        let reg = unsafe { sys::get_registry() };

        let property_name = reg.c_string(name);
        let class_name = reg.c_string(&T::class_name());

        sys::GDNativePropertyInfo {
            type_: Self::variant_type() as u32,
            name: property_name,
            class_name,
            hint: 0,
            hint_string: ptr::null_mut(),
            usage: 7, // Default, TODO generate global enums
        }
    }
}
