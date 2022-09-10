#![allow(dead_code)] // FIXME

use crate::private::as_storage;
use crate::storage::InstanceStorage;
use crate::traits::*;
use std::any::Any;

use once_cell::unsync::Lazy;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ptr;
use std::sync::Mutex;

use crate::Base;
use gdext_sys as sys;
use sys::interface_fn;

#[derive(Debug)]
pub struct ClassPlugin {
    pub class_name: &'static str,
    pub component: PluginComponent,
}

/// Type-erased function object, holding a `register_class` function.
#[derive(Copy, Clone)]
pub struct ErasedRegisterFn {
    // Wrapper needed because Debug can't be derived on function pointers with reference parameters, so this won't work:
    // pub type ErasedRegisterFn = fn(&mut dyn std::any::Any);
    // (see https://stackoverflow.com/q/53380040)
    pub raw: fn(&mut dyn Any),
}

/// Type-erased function object, holding a `init` function.
#[derive(Copy, Clone)]
pub struct ErasedInitFn {
    // Wrapper needed because Debug can't be derived on function pointers with reference parameters, so this won't work:
    // pub type ErasedRegisterFn = fn(&mut dyn std::any::Any);
    // (see https://stackoverflow.com/q/53380040)
    pub raw: fn(Box<dyn Any>) -> Box<dyn Any>,
}

impl std::fmt::Debug for ErasedRegisterFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:0>16x}", self.raw as u64)
    }
}
impl std::fmt::Debug for ErasedInitFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:0>16x}", self.raw as u64)
    }
}

#[derive(Debug, Clone)]
pub enum PluginComponent {
    /// Class definition itself, must always be available
    ClassDef {
        base_class_name: &'static str,

        /// Godot low-level`create` function, wired up to library-generated `init`
        generated_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDNativeObjectPtr,
        >,

        /// Type-erased version of `init`
        generated_erased_init_fn: Option<ErasedInitFn>,

        free_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            instance: sys::GDExtensionClassInstancePtr,
        ),
    },

    /// Collected from `#[godot_api] impl MyClass`
    UserMethodBinds {
        /// Callback to library-generated function which registers functions in the `impl`
        ///
        /// Always present since that's the entire point of this `impl` block.
        generated_register_fn: ErasedRegisterFn,
    },

    /// Collected from `#[godot_api] impl GodotMethods for MyClass`
    UserVirtuals {
        /// Callback to user-defined `register_class` function
        user_register_fn: Option<ErasedRegisterFn>,

        /// Godot low-level`create` function, wired up to the user's `init`
        user_create_fn: Option<
            unsafe extern "C" fn(
                _class_userdata: *mut std::ffi::c_void, //
            ) -> sys::GDNativeObjectPtr,
        >,

        /// Type-erased version of `init`
        user_erased_init_fn: Option<ErasedInitFn>,

        /// User-defined `to_string` function
        user_to_string_fn: Option<
            unsafe extern "C" fn(
                instance: sys::GDExtensionClassInstancePtr,
                out_string: sys::GDNativeStringPtr,
            ),
        >,

        /// Callback for other virtuals
        get_virtual_fn: unsafe extern "C" fn(
            _class_user_data: *mut std::ffi::c_void,
            p_name: *const std::os::raw::c_char,
        ) -> sys::GDNativeExtensionClassCallVirtual,
    },
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Information that is stored at runtime for each class, even after plugin components are unloaded.
#[derive(Clone)]
pub(crate) struct ClassRuntimeInfo {
    /// `create` function -- either user/library generated or not available
    /*pub create_fn: Option<
        unsafe extern "C" fn(
            _class_userdata: *mut std::ffi::c_void, //
        ) -> sys::GDNativeObjectPtr,
    >,*/
    pub erased_init_fn: Option<ErasedInitFn>,
}

impl ClassRuntimeInfo {
    fn insert(class_name: ClassName, info: Self) {
        let mut guard = GDEXT_CLASS_RUNTIME_MAP.lock().unwrap();
        if guard.insert(class_name.clone(), info).is_some() {
            panic!("class `{}` already added to runtime", class_name);
        }
    }

    pub(crate) fn get(class_name: &ClassName) -> Self {
        let guard = GDEXT_CLASS_RUNTIME_MAP.lock().unwrap();
        guard
            .get(class_name)
            .expect("class not added to runtime")
            .clone()
    }
}

// TODO: RwLock and not Mutex, because this is only populated at loading time and then no longer accessed mutably
// TODO: move to user-data ptr per class
static GDEXT_CLASS_RUNTIME_MAP: Mutex<Lazy<HashMap<ClassName, ClassRuntimeInfo>>> =
    Mutex::new(Lazy::new(|| HashMap::new()));

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct ClassRegistrationInfo {
    class_name: ClassName,
    parent_class_name: Option<ClassName>,
    generated_register_fn: Option<ErasedRegisterFn>,
    user_register_fn: Option<ErasedRegisterFn>,
    init_fn: Option<ErasedInitFn>,
    godot_params: sys::GDNativeExtensionClassCreationInfo,
}

pub fn register_class<T: GodotMethods + GodotDefault>() {
    // TODO: provide overloads with only some trait impls

    println!("Manually register class {}", std::any::type_name::<T>());

    let godot_params = sys::GDNativeExtensionClassCreationInfo {
        set_func: None,
        get_func: None,
        get_property_list_func: None,
        free_property_list_func: None,
        property_can_revert_func: None,
        property_get_revert_func: None,
        notification_func: None,
        to_string_func: Some(callbacks::to_string::<T>),
        reference_func: Some(callbacks::reference::<T>),
        unreference_func: Some(callbacks::unreference::<T>),
        create_instance_func: Some(callbacks::create::<T>),
        free_instance_func: Some(callbacks::free::<T>),
        get_virtual_func: Some(callbacks::get_virtual::<T>),
        get_rid_func: None,
        class_userdata: ptr::null_mut(), // will be passed to create fn, but global per class
    };

    register_class_raw(ClassRegistrationInfo {
        class_name: ClassName::new::<T>(),
        parent_class_name: Some(ClassName::new::<T::Base>()),
        generated_register_fn: None,
        user_register_fn: Some(ErasedRegisterFn {
            raw: callbacks::register_class_by_builder::<T>,
        }),
        init_fn: Some(ErasedInitFn {
            raw: callbacks::erased_init::<T>,
        }),
        godot_params,
    });
}

/// Lets Godot know about all classes that have self-registered through the plugin system.
pub fn auto_register_classes() {
    println!("Auto-register classes...");

    // Note: many errors are already caught by the compiler, before this runtime validation even takes place:
    // * missing #[derive(GodotClass)] or impl GodotClass for T
    // * duplicate impl GodotInit for T
    //

    let mut map = HashMap::<ClassName, ClassRegistrationInfo>::new();

    crate::private::iterate_plugins(|elem: &ClassPlugin| {
        //println!("* Plugin: {elem:#?}");

        let name = ClassName::from_static(elem.class_name);
        let class_info = map
            .entry(name.clone())
            .or_insert_with(|| default_creation_info(name));

        fill_class_info(elem.component.clone(), class_info);
    });

    println!("Class-map: {map:#?}");

    for info in map.into_values() {
        println!(">> Reg class:   {}", info.class_name);
        register_class_raw(info);
    }

    println!("All classes auto-registered.");
}

fn fill_class_info(component: PluginComponent, c: &mut ClassRegistrationInfo) {
    // println!("|   reg (before):    {c:?}");
    // println!("|   comp:            {component:?}");
    match component {
        PluginComponent::ClassDef {
            base_class_name,
            generated_create_fn,
            generated_erased_init_fn,
            free_fn,
        } => {
            c.parent_class_name = Some(ClassName::from_static(base_class_name));
            fill_into(
                &mut c.godot_params.create_instance_func,
                generated_create_fn,
            );
            fill_into(&mut c.init_fn, generated_erased_init_fn);
            c.godot_params.free_instance_func = Some(free_fn);
        }

        PluginComponent::UserMethodBinds {
            generated_register_fn,
        } => {
            c.generated_register_fn = Some(generated_register_fn);
        }

        PluginComponent::UserVirtuals {
            user_register_fn,
            user_create_fn,
            user_erased_init_fn,
            user_to_string_fn,
            get_virtual_fn,
        } => {
            c.user_register_fn = user_register_fn;
            fill_into(&mut c.godot_params.create_instance_func, user_create_fn);
            fill_into(&mut c.init_fn, user_erased_init_fn);
            c.godot_params.to_string_func = user_to_string_fn;
            c.godot_params.get_virtual_func = Some(get_virtual_fn);
        }
    }
    // println!("|   reg (after):     {c:?}");
    // println!();
}

fn fill_into<T>(dst: &mut Option<T>, src: Option<T>) {
    match (dst, src) {
        (dst @ None, src) => *dst = src,
        (Some(_), Some(_)) => panic!("option already filled"),
        (Some(_), None) => { /* do nothing */ }
    }
}

fn register_class_raw(info: ClassRegistrationInfo) {
    // First register class...
    unsafe {
        interface_fn!(classdb_register_extension_class)(
            sys::get_library(),
            info.class_name.c_str(),
            info.parent_class_name
                .expect("class defined (parent_class_name)")
                .c_str(),
            ptr::addr_of!(info.godot_params),
        );
    }

    // ...then custom symbols...

    //let mut class_builder = crate::builder::ClassBuilder::<?>::new();
    let mut class_builder = 0; // TODO dummy argument; see callbacks

    // First call generated (proc-macro) registration function, then user-defined one.
    // This mimics the intuition that proc-macros are running "before" normal runtime code.
    if let Some(register_fn) = info.generated_register_fn {
        (register_fn.raw)(&mut class_builder);
    }
    if let Some(register_fn) = info.user_register_fn {
        (register_fn.raw)(&mut class_builder);
    }

    // ...and finally the runtime map
    ClassRuntimeInfo::insert(
        info.class_name,
        ClassRuntimeInfo {
            erased_init_fn: info.init_fn,
        },
    );
}

/// Utility to convert `String` to C `const char*`.
/// Cannot be a function since the backing string must be retained.
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub(crate) struct ClassName {
    backing: String,
}

impl ClassName {
    pub fn new<T: GodotClass>() -> Self {
        Self {
            backing: format!("{}\0", T::class_name()),
        }
    }

    fn from_static(string: &'static str) -> Self {
        Self {
            backing: format!("{}\0", string),
        }
    }

    pub fn c_str(&self) -> *const std::os::raw::c_char {
        self.backing.as_ptr() as *const _
    }
}

impl Display for ClassName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.backing[..self.backing.len() - 1])
    }
}

pub mod callbacks {
    use super::*;
    use crate::builder::ClassBuilder;

    pub unsafe extern "C" fn create<T: GodotClass>(
        _class_userdata: *mut std::ffi::c_void,
    ) -> sys::GDNativeObjectPtr {
        let class_name = ClassName::new::<T>();
        let base_class_name = ClassName::new::<T::Base>();

        println!(">>>>> CREATE: {}", class_name.backing);

        let base = interface_fn!(classdb_construct_object)(base_class_name.c_str());
        let instance = InstanceStorage::<T>::construct_uninit(base);
        let instance_ptr = instance.into_raw();
        let instance_ptr = instance_ptr as *mut std::ffi::c_void; // TODO GDExtensionClassInstancePtr

        interface_fn!(object_set_instance)(base, class_name.c_str(), instance_ptr);

        let binding_data_callbacks = crate::storage::nop_instance_callbacks();

        interface_fn!(object_set_instance_binding)(
            base,
            sys::get_library(),
            instance_ptr,
            &binding_data_callbacks,
        );

        base
    }

    pub unsafe extern "C" fn free<T: GodotClass>(
        _class_user_data: *mut std::ffi::c_void,
        instance: sys::GDExtensionClassInstancePtr,
    ) {
        let storage = as_storage::<T>(instance);
        storage.mark_destroyed_by_godot();
        Box::from_raw(storage as *mut InstanceStorage<_>); // aka. drop
    }

    pub unsafe extern "C" fn get_virtual<T: UserVirtuals>(
        _class_user_data: *mut std::ffi::c_void,
        p_name: *const std::os::raw::c_char,
    ) -> sys::GDNativeExtensionClassCallVirtual {
        let name = std::ffi::CStr::from_ptr(p_name);
        T::virtual_call(name.to_str().expect("T::virtual_call"))
    }

    pub unsafe extern "C" fn to_string<T: GodotMethods>(
        instance: sys::GDExtensionClassInstancePtr,
        out_string: sys::GDNativeStringPtr,
    ) {
        // Lazy/late init is necessary, if the GDScript instantiates a T using `T.new()` and directly uses its
        // string representation, before get_mut_lateinit() is called. Type is erased because T might not
        // statically implement the GodotInit trait (and if it doesn't, it will already be initialized).
        let storage = as_storage::<T>(instance);
        let instance = storage.get_dyn_lateinit();
        let string = GodotMethods::to_string(instance);

        // Transfer ownership to Godot, disable destructor
        string.write_string_sys(out_string);
        std::mem::forget(string);
    }

    pub unsafe extern "C" fn reference<T: GodotClass>(instance: sys::GDExtensionClassInstancePtr) {
        let storage = as_storage::<T>(instance);
        storage.on_inc_ref();
    }

    pub unsafe extern "C" fn unreference<T: GodotClass>(
        instance: sys::GDExtensionClassInstancePtr,
    ) {
        let storage = as_storage::<T>(instance);
        storage.on_dec_ref();
    }

    // Safe, higher-level methods

    /// Abstracts the `GodotInit` away, for contexts where this trait bound is not statically available
    pub fn erased_init<T: GodotDefault>(base: Box<dyn Any>) -> Box<dyn Any> {
        let concrete = base
            .downcast::<Base<<T as GodotClass>::Base>>()
            .expect("erased_init: bad type erasure");
        let extracted: Base<_> = sys::unbox(concrete);

        let instance = T::godot_default(extracted);
        Box::new(instance)
    }

    pub fn register_class_by_builder<T: GodotClass + GodotMethods>(_class_builder: &mut dyn Any) {
        // TODO use actual argument, once class builder carries state
        // let class_builder = class_builder
        //     .downcast_mut::<ClassBuilder<T>>()
        //     .expect("bad type erasure");

        let mut class_builder = ClassBuilder::new();
        T::register_class(&mut class_builder);
    }

    pub fn register_user_binds<T: GodotClass + UserMethodBinds>(_class_builder: &mut dyn Any) {
        // let class_builder = class_builder
        //     .downcast_mut::<ClassBuilder<T>>()
        //     .expect("bad type erasure");

        //T::register_methods(class_builder);
        T::register_methods();
    }
}

// Substitute for Default impl
// Yes, bindgen can implement Default, but only for _all_ types (with single exceptions).
// For FFI types, it's better to have explicit initialization in the general case though.
fn default_creation_info(class_name: ClassName) -> ClassRegistrationInfo {
    ClassRegistrationInfo {
        class_name,
        parent_class_name: None,
        generated_register_fn: None,
        user_register_fn: None,
        init_fn: None,
        godot_params: sys::GDNativeExtensionClassCreationInfo {
            set_func: None,
            get_func: None,
            get_property_list_func: None,
            free_property_list_func: None,
            property_can_revert_func: None,
            property_get_revert_func: None,
            notification_func: None,
            to_string_func: None,
            reference_func: None,
            unreference_func: None,
            create_instance_func: None,
            free_instance_func: None,
            get_virtual_func: None,
            get_rid_func: None,
            class_userdata: ptr::null_mut(),
        },
    }
}
