pub type GDExtensionVariantType = std::ffi::c_int;
pub const GDEXTENSION_VARIANT_TYPE_NIL: GDExtensionVariantType = 0;
pub const GDEXTENSION_VARIANT_TYPE_BOOL: GDExtensionVariantType = 1;
pub const GDEXTENSION_VARIANT_TYPE_INT: GDExtensionVariantType = 2;
pub const GDEXTENSION_VARIANT_TYPE_FLOAT: GDExtensionVariantType = 3;
pub const GDEXTENSION_VARIANT_TYPE_STRING: GDExtensionVariantType = 4;
pub const GDEXTENSION_VARIANT_TYPE_VECTOR2: GDExtensionVariantType = 5;
pub const GDEXTENSION_VARIANT_TYPE_VECTOR2I: GDExtensionVariantType = 6;
pub const GDEXTENSION_VARIANT_TYPE_RECT2: GDExtensionVariantType = 7;
pub const GDEXTENSION_VARIANT_TYPE_RECT2I: GDExtensionVariantType = 8;
pub const GDEXTENSION_VARIANT_TYPE_VECTOR3: GDExtensionVariantType = 9;
pub const GDEXTENSION_VARIANT_TYPE_VECTOR3I: GDExtensionVariantType = 10;
pub const GDEXTENSION_VARIANT_TYPE_TRANSFORM2D: GDExtensionVariantType = 11;
pub const GDEXTENSION_VARIANT_TYPE_VECTOR4: GDExtensionVariantType = 12;
pub const GDEXTENSION_VARIANT_TYPE_VECTOR4I: GDExtensionVariantType = 13;
pub const GDEXTENSION_VARIANT_TYPE_PLANE: GDExtensionVariantType = 14;
pub const GDEXTENSION_VARIANT_TYPE_QUATERNION: GDExtensionVariantType = 15;
pub const GDEXTENSION_VARIANT_TYPE_AABB: GDExtensionVariantType = 16;
pub const GDEXTENSION_VARIANT_TYPE_BASIS: GDExtensionVariantType = 17;
pub const GDEXTENSION_VARIANT_TYPE_TRANSFORM3D: GDExtensionVariantType = 18;
pub const GDEXTENSION_VARIANT_TYPE_PROJECTION: GDExtensionVariantType = 19;
pub const GDEXTENSION_VARIANT_TYPE_COLOR: GDExtensionVariantType = 20;
pub const GDEXTENSION_VARIANT_TYPE_STRING_NAME: GDExtensionVariantType = 21;
pub const GDEXTENSION_VARIANT_TYPE_NODE_PATH: GDExtensionVariantType = 22;
pub const GDEXTENSION_VARIANT_TYPE_RID: GDExtensionVariantType = 23;
pub const GDEXTENSION_VARIANT_TYPE_OBJECT: GDExtensionVariantType = 24;
pub const GDEXTENSION_VARIANT_TYPE_CALLABLE: GDExtensionVariantType = 25;
pub const GDEXTENSION_VARIANT_TYPE_SIGNAL: GDExtensionVariantType = 26;
pub const GDEXTENSION_VARIANT_TYPE_DICTIONARY: GDExtensionVariantType = 27;
pub const GDEXTENSION_VARIANT_TYPE_ARRAY: GDExtensionVariantType = 28;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_BYTE_ARRAY: GDExtensionVariantType = 29;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_INT32_ARRAY: GDExtensionVariantType = 30;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_INT64_ARRAY: GDExtensionVariantType = 31;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_FLOAT32_ARRAY: GDExtensionVariantType = 32;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_FLOAT64_ARRAY: GDExtensionVariantType = 33;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_STRING_ARRAY: GDExtensionVariantType = 34;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_VECTOR2_ARRAY: GDExtensionVariantType = 35;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_VECTOR3_ARRAY: GDExtensionVariantType = 36;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_COLOR_ARRAY: GDExtensionVariantType = 37;
pub const GDEXTENSION_VARIANT_TYPE_PACKED_VECTOR4_ARRAY: GDExtensionVariantType = 38;
pub const GDEXTENSION_VARIANT_TYPE_VARIANT_MAX: GDExtensionVariantType = 39;
pub type GDExtensionVariantOperator = std::ffi::c_int;
pub const GDEXTENSION_VARIANT_OP_EQUAL: GDExtensionVariantOperator = 0;
pub const GDEXTENSION_VARIANT_OP_NOT_EQUAL: GDExtensionVariantOperator = 1;
pub const GDEXTENSION_VARIANT_OP_LESS: GDExtensionVariantOperator = 2;
pub const GDEXTENSION_VARIANT_OP_LESS_EQUAL: GDExtensionVariantOperator = 3;
pub const GDEXTENSION_VARIANT_OP_GREATER: GDExtensionVariantOperator = 4;
pub const GDEXTENSION_VARIANT_OP_GREATER_EQUAL: GDExtensionVariantOperator = 5;
pub const GDEXTENSION_VARIANT_OP_ADD: GDExtensionVariantOperator = 6;
pub const GDEXTENSION_VARIANT_OP_SUBTRACT: GDExtensionVariantOperator = 7;
pub const GDEXTENSION_VARIANT_OP_MULTIPLY: GDExtensionVariantOperator = 8;
pub const GDEXTENSION_VARIANT_OP_DIVIDE: GDExtensionVariantOperator = 9;
pub const GDEXTENSION_VARIANT_OP_NEGATE: GDExtensionVariantOperator = 10;
pub const GDEXTENSION_VARIANT_OP_POSITIVE: GDExtensionVariantOperator = 11;
pub const GDEXTENSION_VARIANT_OP_MODULE: GDExtensionVariantOperator = 12;
pub const GDEXTENSION_VARIANT_OP_POWER: GDExtensionVariantOperator = 13;
pub const GDEXTENSION_VARIANT_OP_SHIFT_LEFT: GDExtensionVariantOperator = 14;
pub const GDEXTENSION_VARIANT_OP_SHIFT_RIGHT: GDExtensionVariantOperator = 15;
pub const GDEXTENSION_VARIANT_OP_BIT_AND: GDExtensionVariantOperator = 16;
pub const GDEXTENSION_VARIANT_OP_BIT_OR: GDExtensionVariantOperator = 17;
pub const GDEXTENSION_VARIANT_OP_BIT_XOR: GDExtensionVariantOperator = 18;
pub const GDEXTENSION_VARIANT_OP_BIT_NEGATE: GDExtensionVariantOperator = 19;
pub const GDEXTENSION_VARIANT_OP_AND: GDExtensionVariantOperator = 20;
pub const GDEXTENSION_VARIANT_OP_OR: GDExtensionVariantOperator = 21;
pub const GDEXTENSION_VARIANT_OP_XOR: GDExtensionVariantOperator = 22;
pub const GDEXTENSION_VARIANT_OP_NOT: GDExtensionVariantOperator = 23;
pub const GDEXTENSION_VARIANT_OP_IN: GDExtensionVariantOperator = 24;
pub const GDEXTENSION_VARIANT_OP_MAX: GDExtensionVariantOperator = 25;
pub type GDExtensionVariantPtr = *mut std::ffi::c_void;
pub type GDExtensionConstVariantPtr = *mut std::ffi::c_void;
pub type GDExtensionUninitializedVariantPtr = *mut std::ffi::c_void;
pub type GDExtensionStringNamePtr = *mut std::ffi::c_void;
pub type GDExtensionConstStringNamePtr = *mut std::ffi::c_void;
pub type GDExtensionUninitializedStringNamePtr = *mut std::ffi::c_void;
pub type GDExtensionStringPtr = *mut std::ffi::c_void;
pub type GDExtensionConstStringPtr = *mut std::ffi::c_void;
pub type GDExtensionUninitializedStringPtr = *mut std::ffi::c_void;
pub type GDExtensionObjectPtr = *mut std::ffi::c_void;
pub type GDExtensionConstObjectPtr = *mut std::ffi::c_void;
pub type GDExtensionUninitializedObjectPtr = *mut std::ffi::c_void;
pub type GDExtensionTypePtr = *mut std::ffi::c_void;
pub type GDExtensionConstTypePtr = *mut std::ffi::c_void;
pub type GDExtensionUninitializedTypePtr = *mut std::ffi::c_void;
pub type GDExtensionMethodBindPtr = *mut std::ffi::c_void;
pub type GDExtensionInt = i64;
pub type GDExtensionBool = u8;
pub type GDObjectInstanceID = u64;
pub type GDExtensionRefPtr = *mut std::ffi::c_void;
pub type GDExtensionConstRefPtr = *mut std::ffi::c_void;
pub type GDExtensionCallErrorType = std::ffi::c_int;
pub const GDEXTENSION_CALL_OK: GDExtensionCallErrorType = 0;
pub const GDEXTENSION_CALL_ERROR_INVALID_METHOD: GDExtensionCallErrorType = 1;
pub const GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT: GDExtensionCallErrorType = 2;
pub const GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS: GDExtensionCallErrorType = 3;
pub const GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS: GDExtensionCallErrorType = 4;
pub const GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL: GDExtensionCallErrorType = 5;
pub const GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST: GDExtensionCallErrorType = 6;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionCallError {
    pub error: GDExtensionCallErrorType,
    pub argument: i32,
    pub expected: i32,
}
pub type GDExtensionVariantFromTypeConstructorFunc =
    unsafe extern "C" fn(GDExtensionUninitializedVariantPtr, GDExtensionTypePtr) -> ();
pub type GDExtensionTypeFromVariantConstructorFunc =
    unsafe extern "C" fn(GDExtensionUninitializedTypePtr, GDExtensionVariantPtr) -> ();
pub type GDExtensionVariantGetInternalPtrFunc =
    unsafe extern "C" fn(GDExtensionVariantPtr) -> *mut ();
pub type GDExtensionPtrOperatorEvaluator = unsafe extern "C" fn(
    p_left: GDExtensionConstTypePtr,
    p_right: GDExtensionConstTypePtr,
    r_result: GDExtensionTypePtr,
) -> ();
pub type GDExtensionPtrBuiltInMethod = unsafe extern "C" fn(
    p_base: GDExtensionTypePtr,
    p_args: *const GDExtensionConstTypePtr,
    r_return: GDExtensionTypePtr,
    p_argument_count: std::ffi::c_int,
) -> ();
pub type GDExtensionPtrConstructor = unsafe extern "C" fn(
    p_base: GDExtensionUninitializedTypePtr,
    p_args: *const GDExtensionConstTypePtr,
) -> ();
pub type GDExtensionPtrDestructor = unsafe extern "C" fn(p_base: GDExtensionTypePtr) -> ();
pub type GDExtensionPtrSetter =
    unsafe extern "C" fn(p_base: GDExtensionTypePtr, p_value: GDExtensionConstTypePtr) -> ();
pub type GDExtensionPtrGetter =
    unsafe extern "C" fn(p_base: GDExtensionConstTypePtr, r_value: GDExtensionTypePtr) -> ();
pub type GDExtensionPtrIndexedSetter = unsafe extern "C" fn(
    p_base: GDExtensionTypePtr,
    p_index: GDExtensionInt,
    p_value: GDExtensionConstTypePtr,
) -> ();
pub type GDExtensionPtrIndexedGetter = unsafe extern "C" fn(
    p_base: GDExtensionConstTypePtr,
    p_index: GDExtensionInt,
    r_value: GDExtensionTypePtr,
) -> ();
pub type GDExtensionPtrKeyedSetter = unsafe extern "C" fn(
    p_base: GDExtensionTypePtr,
    p_key: GDExtensionConstTypePtr,
    p_value: GDExtensionConstTypePtr,
) -> ();
pub type GDExtensionPtrKeyedGetter = unsafe extern "C" fn(
    p_base: GDExtensionConstTypePtr,
    p_key: GDExtensionConstTypePtr,
    r_value: GDExtensionTypePtr,
) -> ();
pub type GDExtensionPtrKeyedChecker = unsafe extern "C" fn(
    p_base: GDExtensionConstVariantPtr,
    p_key: GDExtensionConstVariantPtr,
) -> u32;
pub type GDExtensionPtrUtilityFunction = unsafe extern "C" fn(
    r_return: GDExtensionTypePtr,
    p_args: *const GDExtensionConstTypePtr,
    p_argument_count: std::ffi::c_int,
) -> ();
pub type GDExtensionClassConstructor = unsafe extern "C" fn() -> GDExtensionObjectPtr;
pub type GDExtensionInstanceBindingCreateCallback =
    unsafe extern "C" fn(p_token: *mut (), p_instance: *mut ()) -> *mut ();
pub type GDExtensionInstanceBindingFreeCallback =
    unsafe extern "C" fn(p_token: *mut (), p_instance: *mut (), p_binding: *mut ()) -> ();
pub type GDExtensionInstanceBindingReferenceCallback = unsafe extern "C" fn(
    p_token: *mut (),
    p_binding: *mut (),
    p_reference: GDExtensionBool,
) -> GDExtensionBool;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionInstanceBindingCallbacks {
    pub create_callback: GDExtensionInstanceBindingCreateCallback,
    pub free_callback: GDExtensionInstanceBindingFreeCallback,
    pub reference_callback: GDExtensionInstanceBindingReferenceCallback,
}
pub type GDExtensionClassInstancePtr = *mut std::ffi::c_void;
pub type GDExtensionClassSet = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_name: GDExtensionConstStringNamePtr,
    p_value: GDExtensionConstVariantPtr,
) -> GDExtensionBool;
pub type GDExtensionClassGet = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_name: GDExtensionConstStringNamePtr,
    r_ret: GDExtensionVariantPtr,
) -> GDExtensionBool;
pub type GDExtensionClassGetRID =
    unsafe extern "C" fn(p_instance: GDExtensionClassInstancePtr) -> u64;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionPropertyInfo {
    pub type_: GDExtensionVariantType,
    pub name: GDExtensionStringNamePtr,
    pub class_name: GDExtensionStringNamePtr,
    pub hint: u32,
    pub hint_string: GDExtensionStringPtr,
    pub usage: u32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionMethodInfo {
    pub name: GDExtensionStringNamePtr,
    pub return_value: GDExtensionPropertyInfo,
    pub flags: u32,
    pub id: i32,
    pub argument_count: u32,
    pub arguments: *mut GDExtensionPropertyInfo,
    pub default_argument_count: u32,
    pub default_arguments: *mut GDExtensionVariantPtr,
}
pub type GDExtensionClassGetPropertyList = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    r_count: *mut u32,
) -> *const GDExtensionPropertyInfo;
pub type GDExtensionClassFreePropertyList = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_list: *const GDExtensionPropertyInfo,
) -> ();
pub type GDExtensionClassFreePropertyList2 = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_list: *const GDExtensionPropertyInfo,
    p_count: u32,
) -> ();
pub type GDExtensionClassPropertyCanRevert = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_name: GDExtensionConstStringNamePtr,
) -> GDExtensionBool;
pub type GDExtensionClassPropertyGetRevert = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_name: GDExtensionConstStringNamePtr,
    r_ret: GDExtensionVariantPtr,
) -> GDExtensionBool;
pub type GDExtensionClassValidateProperty = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_property: *mut GDExtensionPropertyInfo,
) -> GDExtensionBool;
pub type GDExtensionClassNotification =
    unsafe extern "C" fn(p_instance: GDExtensionClassInstancePtr, p_what: i32) -> ();
pub type GDExtensionClassNotification2 = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_what: i32,
    p_reversed: GDExtensionBool,
) -> ();
pub type GDExtensionClassToString = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    r_is_valid: *mut GDExtensionBool,
    p_out: GDExtensionStringPtr,
) -> ();
pub type GDExtensionClassReference =
    unsafe extern "C" fn(p_instance: GDExtensionClassInstancePtr) -> ();
pub type GDExtensionClassUnreference =
    unsafe extern "C" fn(p_instance: GDExtensionClassInstancePtr) -> ();
pub type GDExtensionClassCallVirtual = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstTypePtr,
    r_ret: GDExtensionTypePtr,
) -> ();
pub type GDExtensionClassCreateInstance =
    unsafe extern "C" fn(p_class_userdata: *mut ()) -> GDExtensionObjectPtr;
pub type GDExtensionClassCreateInstance2 = unsafe extern "C" fn(
    p_class_userdata: *mut (),
    p_notify_postinitialize: GDExtensionBool,
) -> GDExtensionObjectPtr;
pub type GDExtensionClassFreeInstance =
    unsafe extern "C" fn(p_class_userdata: *mut (), p_instance: GDExtensionClassInstancePtr) -> ();
pub type GDExtensionClassRecreateInstance = unsafe extern "C" fn(
    p_class_userdata: *mut (),
    p_object: GDExtensionObjectPtr,
) -> GDExtensionClassInstancePtr;
pub type GDExtensionClassGetVirtual = unsafe extern "C" fn(
    p_class_userdata: *mut (),
    p_name: GDExtensionConstStringNamePtr,
) -> GDExtensionClassCallVirtual;
pub type GDExtensionClassGetVirtual2 = unsafe extern "C" fn(
    p_class_userdata: *mut (),
    p_name: GDExtensionConstStringNamePtr,
    p_hash: u32,
) -> GDExtensionClassCallVirtual;
pub type GDExtensionClassGetVirtualCallData = unsafe extern "C" fn(
    p_class_userdata: *mut (),
    p_name: GDExtensionConstStringNamePtr,
) -> *mut ();
pub type GDExtensionClassGetVirtualCallData2 = unsafe extern "C" fn(
    p_class_userdata: *mut (),
    p_name: GDExtensionConstStringNamePtr,
    p_hash: u32,
) -> *mut ();
pub type GDExtensionClassCallVirtualWithData = unsafe extern "C" fn(
    p_instance: GDExtensionClassInstancePtr,
    p_name: GDExtensionConstStringNamePtr,
    p_virtual_call_userdata: *mut (),
    p_args: *const GDExtensionConstTypePtr,
    r_ret: GDExtensionTypePtr,
) -> ();
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassCreationInfo {
    pub is_virtual: GDExtensionBool,
    pub is_abstract: GDExtensionBool,
    pub set_func: GDExtensionClassSet,
    pub get_func: GDExtensionClassGet,
    pub get_property_list_func: GDExtensionClassGetPropertyList,
    pub free_property_list_func: GDExtensionClassFreePropertyList,
    pub property_can_revert_func: GDExtensionClassPropertyCanRevert,
    pub property_get_revert_func: GDExtensionClassPropertyGetRevert,
    pub notification_func: GDExtensionClassNotification,
    pub to_string_func: GDExtensionClassToString,
    pub reference_func: GDExtensionClassReference,
    pub unreference_func: GDExtensionClassUnreference,
    pub create_instance_func: GDExtensionClassCreateInstance,
    pub free_instance_func: GDExtensionClassFreeInstance,
    pub get_virtual_func: GDExtensionClassGetVirtual,
    pub get_rid_func: GDExtensionClassGetRID,
    pub class_userdata: *mut (),
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassCreationInfo2 {
    pub is_virtual: GDExtensionBool,
    pub is_abstract: GDExtensionBool,
    pub is_exposed: GDExtensionBool,
    pub set_func: GDExtensionClassSet,
    pub get_func: GDExtensionClassGet,
    pub get_property_list_func: GDExtensionClassGetPropertyList,
    pub free_property_list_func: GDExtensionClassFreePropertyList,
    pub property_can_revert_func: GDExtensionClassPropertyCanRevert,
    pub property_get_revert_func: GDExtensionClassPropertyGetRevert,
    pub validate_property_func: GDExtensionClassValidateProperty,
    pub notification_func: GDExtensionClassNotification2,
    pub to_string_func: GDExtensionClassToString,
    pub reference_func: GDExtensionClassReference,
    pub unreference_func: GDExtensionClassUnreference,
    pub create_instance_func: GDExtensionClassCreateInstance,
    pub free_instance_func: GDExtensionClassFreeInstance,
    pub recreate_instance_func: GDExtensionClassRecreateInstance,
    pub get_virtual_func: GDExtensionClassGetVirtual,
    pub get_virtual_call_data_func: GDExtensionClassGetVirtualCallData,
    pub call_virtual_with_data_func: GDExtensionClassCallVirtualWithData,
    pub get_rid_func: GDExtensionClassGetRID,
    pub class_userdata: *mut (),
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassCreationInfo3 {
    pub is_virtual: GDExtensionBool,
    pub is_abstract: GDExtensionBool,
    pub is_exposed: GDExtensionBool,
    pub is_runtime: GDExtensionBool,
    pub set_func: GDExtensionClassSet,
    pub get_func: GDExtensionClassGet,
    pub get_property_list_func: GDExtensionClassGetPropertyList,
    pub free_property_list_func: GDExtensionClassFreePropertyList2,
    pub property_can_revert_func: GDExtensionClassPropertyCanRevert,
    pub property_get_revert_func: GDExtensionClassPropertyGetRevert,
    pub validate_property_func: GDExtensionClassValidateProperty,
    pub notification_func: GDExtensionClassNotification2,
    pub to_string_func: GDExtensionClassToString,
    pub reference_func: GDExtensionClassReference,
    pub unreference_func: GDExtensionClassUnreference,
    pub create_instance_func: GDExtensionClassCreateInstance,
    pub free_instance_func: GDExtensionClassFreeInstance,
    pub recreate_instance_func: GDExtensionClassRecreateInstance,
    pub get_virtual_func: GDExtensionClassGetVirtual,
    pub get_virtual_call_data_func: GDExtensionClassGetVirtualCallData,
    pub call_virtual_with_data_func: GDExtensionClassCallVirtualWithData,
    pub get_rid_func: GDExtensionClassGetRID,
    pub class_userdata: *mut (),
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassCreationInfo4 {
    pub is_virtual: GDExtensionBool,
    pub is_abstract: GDExtensionBool,
    pub is_exposed: GDExtensionBool,
    pub is_runtime: GDExtensionBool,
    pub icon_path: GDExtensionConstStringPtr,
    pub set_func: GDExtensionClassSet,
    pub get_func: GDExtensionClassGet,
    pub get_property_list_func: GDExtensionClassGetPropertyList,
    pub free_property_list_func: GDExtensionClassFreePropertyList2,
    pub property_can_revert_func: GDExtensionClassPropertyCanRevert,
    pub property_get_revert_func: GDExtensionClassPropertyGetRevert,
    pub validate_property_func: GDExtensionClassValidateProperty,
    pub notification_func: GDExtensionClassNotification2,
    pub to_string_func: GDExtensionClassToString,
    pub reference_func: GDExtensionClassReference,
    pub unreference_func: GDExtensionClassUnreference,
    pub create_instance_func: GDExtensionClassCreateInstance2,
    pub free_instance_func: GDExtensionClassFreeInstance,
    pub recreate_instance_func: GDExtensionClassRecreateInstance,
    pub get_virtual_func: GDExtensionClassGetVirtual2,
    pub get_virtual_call_data_func: GDExtensionClassGetVirtualCallData2,
    pub call_virtual_with_data_func: GDExtensionClassCallVirtualWithData,
    pub class_userdata: *mut (),
}
pub type GDExtensionClassCreationInfo5 = GDExtensionClassCreationInfo4;
pub type GDExtensionClassLibraryPtr = *mut std::ffi::c_void;
pub type GDExtensionEditorGetClassesUsedCallback =
    unsafe extern "C" fn(p_packed_string_array: GDExtensionTypePtr) -> ();
pub type GDExtensionClassMethodFlags = std::ffi::c_int;
pub const GDEXTENSION_METHOD_FLAG_NORMAL: GDExtensionClassMethodFlags = 1;
pub const GDEXTENSION_METHOD_FLAG_EDITOR: GDExtensionClassMethodFlags = 2;
pub const GDEXTENSION_METHOD_FLAG_CONST: GDExtensionClassMethodFlags = 4;
pub const GDEXTENSION_METHOD_FLAG_VIRTUAL: GDExtensionClassMethodFlags = 8;
pub const GDEXTENSION_METHOD_FLAG_VARARG: GDExtensionClassMethodFlags = 16;
pub const GDEXTENSION_METHOD_FLAG_STATIC: GDExtensionClassMethodFlags = 32;
pub const GDEXTENSION_METHOD_FLAG_VIRTUAL_REQUIRED: GDExtensionClassMethodFlags = 128;
pub const GDEXTENSION_METHOD_FLAGS_DEFAULT: GDExtensionClassMethodFlags = 1;
pub type GDExtensionClassMethodArgumentMetadata = std::ffi::c_int;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_NONE: GDExtensionClassMethodArgumentMetadata = 0;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8: GDExtensionClassMethodArgumentMetadata =
    1;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16:
    GDExtensionClassMethodArgumentMetadata = 2;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32:
    GDExtensionClassMethodArgumentMetadata = 3;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64:
    GDExtensionClassMethodArgumentMetadata = 4;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8:
    GDExtensionClassMethodArgumentMetadata = 5;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16:
    GDExtensionClassMethodArgumentMetadata = 6;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32:
    GDExtensionClassMethodArgumentMetadata = 7;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64:
    GDExtensionClassMethodArgumentMetadata = 8;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_FLOAT:
    GDExtensionClassMethodArgumentMetadata = 9;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_DOUBLE:
    GDExtensionClassMethodArgumentMetadata = 10;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_CHAR16:
    GDExtensionClassMethodArgumentMetadata = 11;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_CHAR32:
    GDExtensionClassMethodArgumentMetadata = 12;
pub const GDEXTENSION_METHOD_ARGUMENT_METADATA_OBJECT_IS_REQUIRED:
    GDExtensionClassMethodArgumentMetadata = 13;
pub type GDExtensionClassMethodCall = unsafe extern "C" fn(
    method_userdata: *mut (),
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstVariantPtr,
    p_argument_count: GDExtensionInt,
    r_return: GDExtensionVariantPtr,
    r_error: *mut GDExtensionCallError,
) -> ();
pub type GDExtensionClassMethodValidatedCall = unsafe extern "C" fn(
    method_userdata: *mut (),
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstVariantPtr,
    r_return: GDExtensionVariantPtr,
) -> ();
pub type GDExtensionClassMethodPtrCall = unsafe extern "C" fn(
    method_userdata: *mut (),
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstTypePtr,
    r_ret: GDExtensionTypePtr,
) -> ();
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassMethodInfo {
    pub name: GDExtensionStringNamePtr,
    pub method_userdata: *mut (),
    pub call_func: GDExtensionClassMethodCall,
    pub ptrcall_func: GDExtensionClassMethodPtrCall,
    pub method_flags: u32,
    pub has_return_value: GDExtensionBool,
    pub return_value_info: *mut GDExtensionPropertyInfo,
    pub return_value_metadata: GDExtensionClassMethodArgumentMetadata,
    pub argument_count: u32,
    pub arguments_info: *mut GDExtensionPropertyInfo,
    pub arguments_metadata: *mut GDExtensionClassMethodArgumentMetadata,
    pub default_argument_count: u32,
    pub default_arguments: *mut GDExtensionVariantPtr,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassVirtualMethodInfo {
    pub name: GDExtensionStringNamePtr,
    pub method_flags: u32,
    pub return_value: GDExtensionPropertyInfo,
    pub return_value_metadata: GDExtensionClassMethodArgumentMetadata,
    pub argument_count: u32,
    pub arguments: *mut GDExtensionPropertyInfo,
    pub arguments_metadata: *mut GDExtensionClassMethodArgumentMetadata,
}
pub type GDExtensionCallableCustomCall = unsafe extern "C" fn(
    callable_userdata: *mut (),
    p_args: *const GDExtensionConstVariantPtr,
    p_argument_count: GDExtensionInt,
    r_return: GDExtensionVariantPtr,
    r_error: *mut GDExtensionCallError,
) -> ();
pub type GDExtensionCallableCustomIsValid =
    unsafe extern "C" fn(callable_userdata: *mut ()) -> GDExtensionBool;
pub type GDExtensionCallableCustomFree = unsafe extern "C" fn(callable_userdata: *mut ()) -> ();
pub type GDExtensionCallableCustomHash = unsafe extern "C" fn(callable_userdata: *mut ()) -> u32;
pub type GDExtensionCallableCustomEqual = unsafe extern "C" fn(
    callable_userdata_a: *mut (),
    callable_userdata_b: *mut (),
) -> GDExtensionBool;
pub type GDExtensionCallableCustomLessThan = unsafe extern "C" fn(
    callable_userdata_a: *mut (),
    callable_userdata_b: *mut (),
) -> GDExtensionBool;
pub type GDExtensionCallableCustomToString = unsafe extern "C" fn(
    callable_userdata: *mut (),
    r_is_valid: *mut GDExtensionBool,
    r_out: GDExtensionStringPtr,
) -> ();
pub type GDExtensionCallableCustomGetArgumentCount = unsafe extern "C" fn(
    callable_userdata: *mut (),
    r_is_valid: *mut GDExtensionBool,
) -> GDExtensionInt;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionCallableCustomInfo {
    pub callable_userdata: *mut (),
    pub token: *mut (),
    pub object_id: GDObjectInstanceID,
    pub call_func: GDExtensionCallableCustomCall,
    pub is_valid_func: GDExtensionCallableCustomIsValid,
    pub free_func: GDExtensionCallableCustomFree,
    pub hash_func: GDExtensionCallableCustomHash,
    pub equal_func: GDExtensionCallableCustomEqual,
    pub less_than_func: GDExtensionCallableCustomLessThan,
    pub to_string_func: GDExtensionCallableCustomToString,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionCallableCustomInfo2 {
    pub callable_userdata: *mut (),
    pub token: *mut (),
    pub object_id: GDObjectInstanceID,
    pub call_func: GDExtensionCallableCustomCall,
    pub is_valid_func: GDExtensionCallableCustomIsValid,
    pub free_func: GDExtensionCallableCustomFree,
    pub hash_func: GDExtensionCallableCustomHash,
    pub equal_func: GDExtensionCallableCustomEqual,
    pub less_than_func: GDExtensionCallableCustomLessThan,
    pub to_string_func: GDExtensionCallableCustomToString,
    pub get_argument_count_func: GDExtensionCallableCustomGetArgumentCount,
}
pub type GDExtensionScriptInstanceDataPtr = *mut std::ffi::c_void;
pub type GDExtensionScriptInstanceSet = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
    p_value: GDExtensionConstVariantPtr,
) -> GDExtensionBool;
pub type GDExtensionScriptInstanceGet = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
    r_ret: GDExtensionVariantPtr,
) -> GDExtensionBool;
pub type GDExtensionScriptInstanceGetPropertyList =
    unsafe extern "C" fn(
        p_instance: GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const GDExtensionPropertyInfo;
pub type GDExtensionScriptInstanceFreePropertyList = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_list: *const GDExtensionPropertyInfo,
) -> ();
pub type GDExtensionScriptInstanceFreePropertyList2 = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_list: *const GDExtensionPropertyInfo,
    p_count: u32,
) -> ();
pub type GDExtensionScriptInstanceGetClassCategory = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_class_category: *mut GDExtensionPropertyInfo,
) -> GDExtensionBool;
pub type GDExtensionScriptInstanceGetPropertyType = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
    r_is_valid: *mut GDExtensionBool,
) -> GDExtensionVariantType;
pub type GDExtensionScriptInstanceValidateProperty = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_property: *mut GDExtensionPropertyInfo,
) -> GDExtensionBool;
pub type GDExtensionScriptInstancePropertyCanRevert = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
) -> GDExtensionBool;
pub type GDExtensionScriptInstancePropertyGetRevert = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
    r_ret: GDExtensionVariantPtr,
) -> GDExtensionBool;
pub type GDExtensionScriptInstanceGetOwner =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr) -> GDExtensionObjectPtr;
pub type GDExtensionScriptInstancePropertyStateAdd = unsafe extern "C" fn(
    p_name: GDExtensionConstStringNamePtr,
    p_value: GDExtensionConstVariantPtr,
    p_userdata: *mut (),
) -> ();
pub type GDExtensionScriptInstanceGetPropertyState = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_add_func: GDExtensionScriptInstancePropertyStateAdd,
    p_userdata: *mut (),
) -> ();
pub type GDExtensionScriptInstanceGetMethodList =
    unsafe extern "C" fn(
        p_instance: GDExtensionScriptInstanceDataPtr,
        r_count: *mut u32,
    ) -> *const GDExtensionMethodInfo;
pub type GDExtensionScriptInstanceFreeMethodList = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_list: *const GDExtensionMethodInfo,
) -> ();
pub type GDExtensionScriptInstanceFreeMethodList2 = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_list: *const GDExtensionMethodInfo,
    p_count: u32,
) -> ();
pub type GDExtensionScriptInstanceHasMethod = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
) -> GDExtensionBool;
pub type GDExtensionScriptInstanceGetMethodArgumentCount = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_name: GDExtensionConstStringNamePtr,
    r_is_valid: *mut GDExtensionBool,
) -> GDExtensionInt;
pub type GDExtensionScriptInstanceCall = unsafe extern "C" fn(
    p_self: GDExtensionScriptInstanceDataPtr,
    p_method: GDExtensionConstStringNamePtr,
    p_args: *const GDExtensionConstVariantPtr,
    p_argument_count: GDExtensionInt,
    r_return: GDExtensionVariantPtr,
    r_error: *mut GDExtensionCallError,
) -> ();
pub type GDExtensionScriptInstanceNotification =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr, p_what: i32) -> ();
pub type GDExtensionScriptInstanceNotification2 = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    p_what: i32,
    p_reversed: GDExtensionBool,
) -> ();
pub type GDExtensionScriptInstanceToString = unsafe extern "C" fn(
    p_instance: GDExtensionScriptInstanceDataPtr,
    r_is_valid: *mut GDExtensionBool,
    r_out: GDExtensionStringPtr,
) -> ();
pub type GDExtensionScriptInstanceRefCountIncremented =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr) -> ();
pub type GDExtensionScriptInstanceRefCountDecremented =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr) -> GDExtensionBool;
pub type GDExtensionScriptInstanceGetScript =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr) -> GDExtensionObjectPtr;
pub type GDExtensionScriptInstanceIsPlaceholder =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr) -> GDExtensionBool;
pub type GDExtensionScriptLanguagePtr = *mut std::ffi::c_void;
pub type GDExtensionScriptInstanceGetLanguage =
    unsafe extern "C" fn(
        p_instance: GDExtensionScriptInstanceDataPtr,
    ) -> GDExtensionScriptLanguagePtr;
pub type GDExtensionScriptInstanceFree =
    unsafe extern "C" fn(p_instance: GDExtensionScriptInstanceDataPtr) -> ();
pub type GDExtensionScriptInstancePtr = *mut std::ffi::c_void;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionScriptInstanceInfo {
    pub set_func: GDExtensionScriptInstanceSet,
    pub get_func: GDExtensionScriptInstanceGet,
    pub get_property_list_func: GDExtensionScriptInstanceGetPropertyList,
    pub free_property_list_func: GDExtensionScriptInstanceFreePropertyList,
    pub property_can_revert_func: GDExtensionScriptInstancePropertyCanRevert,
    pub property_get_revert_func: GDExtensionScriptInstancePropertyGetRevert,
    pub get_owner_func: GDExtensionScriptInstanceGetOwner,
    pub get_property_state_func: GDExtensionScriptInstanceGetPropertyState,
    pub get_method_list_func: GDExtensionScriptInstanceGetMethodList,
    pub free_method_list_func: GDExtensionScriptInstanceFreeMethodList,
    pub get_property_type_func: GDExtensionScriptInstanceGetPropertyType,
    pub has_method_func: GDExtensionScriptInstanceHasMethod,
    pub call_func: GDExtensionScriptInstanceCall,
    pub notification_func: GDExtensionScriptInstanceNotification,
    pub to_string_func: GDExtensionScriptInstanceToString,
    pub refcount_incremented_func: GDExtensionScriptInstanceRefCountIncremented,
    pub refcount_decremented_func: GDExtensionScriptInstanceRefCountDecremented,
    pub get_script_func: GDExtensionScriptInstanceGetScript,
    pub is_placeholder_func: GDExtensionScriptInstanceIsPlaceholder,
    pub set_fallback_func: GDExtensionScriptInstanceSet,
    pub get_fallback_func: GDExtensionScriptInstanceGet,
    pub get_language_func: GDExtensionScriptInstanceGetLanguage,
    pub free_func: GDExtensionScriptInstanceFree,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionScriptInstanceInfo2 {
    pub set_func: GDExtensionScriptInstanceSet,
    pub get_func: GDExtensionScriptInstanceGet,
    pub get_property_list_func: GDExtensionScriptInstanceGetPropertyList,
    pub free_property_list_func: GDExtensionScriptInstanceFreePropertyList,
    pub get_class_category_func: GDExtensionScriptInstanceGetClassCategory,
    pub property_can_revert_func: GDExtensionScriptInstancePropertyCanRevert,
    pub property_get_revert_func: GDExtensionScriptInstancePropertyGetRevert,
    pub get_owner_func: GDExtensionScriptInstanceGetOwner,
    pub get_property_state_func: GDExtensionScriptInstanceGetPropertyState,
    pub get_method_list_func: GDExtensionScriptInstanceGetMethodList,
    pub free_method_list_func: GDExtensionScriptInstanceFreeMethodList,
    pub get_property_type_func: GDExtensionScriptInstanceGetPropertyType,
    pub validate_property_func: GDExtensionScriptInstanceValidateProperty,
    pub has_method_func: GDExtensionScriptInstanceHasMethod,
    pub call_func: GDExtensionScriptInstanceCall,
    pub notification_func: GDExtensionScriptInstanceNotification2,
    pub to_string_func: GDExtensionScriptInstanceToString,
    pub refcount_incremented_func: GDExtensionScriptInstanceRefCountIncremented,
    pub refcount_decremented_func: GDExtensionScriptInstanceRefCountDecremented,
    pub get_script_func: GDExtensionScriptInstanceGetScript,
    pub is_placeholder_func: GDExtensionScriptInstanceIsPlaceholder,
    pub set_fallback_func: GDExtensionScriptInstanceSet,
    pub get_fallback_func: GDExtensionScriptInstanceGet,
    pub get_language_func: GDExtensionScriptInstanceGetLanguage,
    pub free_func: GDExtensionScriptInstanceFree,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionScriptInstanceInfo3 {
    pub set_func: GDExtensionScriptInstanceSet,
    pub get_func: GDExtensionScriptInstanceGet,
    pub get_property_list_func: GDExtensionScriptInstanceGetPropertyList,
    pub free_property_list_func: GDExtensionScriptInstanceFreePropertyList2,
    pub get_class_category_func: GDExtensionScriptInstanceGetClassCategory,
    pub property_can_revert_func: GDExtensionScriptInstancePropertyCanRevert,
    pub property_get_revert_func: GDExtensionScriptInstancePropertyGetRevert,
    pub get_owner_func: GDExtensionScriptInstanceGetOwner,
    pub get_property_state_func: GDExtensionScriptInstanceGetPropertyState,
    pub get_method_list_func: GDExtensionScriptInstanceGetMethodList,
    pub free_method_list_func: GDExtensionScriptInstanceFreeMethodList2,
    pub get_property_type_func: GDExtensionScriptInstanceGetPropertyType,
    pub validate_property_func: GDExtensionScriptInstanceValidateProperty,
    pub has_method_func: GDExtensionScriptInstanceHasMethod,
    pub get_method_argument_count_func: GDExtensionScriptInstanceGetMethodArgumentCount,
    pub call_func: GDExtensionScriptInstanceCall,
    pub notification_func: GDExtensionScriptInstanceNotification2,
    pub to_string_func: GDExtensionScriptInstanceToString,
    pub refcount_incremented_func: GDExtensionScriptInstanceRefCountIncremented,
    pub refcount_decremented_func: GDExtensionScriptInstanceRefCountDecremented,
    pub get_script_func: GDExtensionScriptInstanceGetScript,
    pub is_placeholder_func: GDExtensionScriptInstanceIsPlaceholder,
    pub set_fallback_func: GDExtensionScriptInstanceSet,
    pub get_fallback_func: GDExtensionScriptInstanceGet,
    pub get_language_func: GDExtensionScriptInstanceGetLanguage,
    pub free_func: GDExtensionScriptInstanceFree,
}
pub type GDExtensionWorkerThreadPoolGroupTask = unsafe extern "C" fn(*mut (), u32) -> ();
pub type GDExtensionWorkerThreadPoolTask = unsafe extern "C" fn(*mut ()) -> ();
pub type GDExtensionInitializationLevel = std::ffi::c_int;
pub const GDEXTENSION_INITIALIZATION_CORE: GDExtensionInitializationLevel = 0;
pub const GDEXTENSION_INITIALIZATION_SERVERS: GDExtensionInitializationLevel = 1;
pub const GDEXTENSION_INITIALIZATION_SCENE: GDExtensionInitializationLevel = 2;
pub const GDEXTENSION_INITIALIZATION_EDITOR: GDExtensionInitializationLevel = 3;
pub const GDEXTENSION_MAX_INITIALIZATION_LEVEL: GDExtensionInitializationLevel = 4;
pub type GDExtensionInitializeCallback =
    unsafe extern "C" fn(p_userdata: *mut (), p_level: GDExtensionInitializationLevel) -> ();
pub type GDExtensionDeinitializeCallback =
    unsafe extern "C" fn(p_userdata: *mut (), p_level: GDExtensionInitializationLevel) -> ();
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionInitialization {
    pub minimum_initialization_level: GDExtensionInitializationLevel,
    pub userdata: *mut (),
    pub initialize: GDExtensionInitializeCallback,
    pub deinitialize: GDExtensionDeinitializeCallback,
}
pub type GDExtensionInterfaceFunctionPtr = unsafe extern "C" fn() -> ();
pub type GDExtensionInterfaceGetProcAddress =
    unsafe extern "C" fn(
        p_function_name: *const std::ffi::c_char,
    ) -> GDExtensionInterfaceFunctionPtr;
pub type GDExtensionInitializationFunction = unsafe extern "C" fn(
    p_get_proc_address: GDExtensionInterfaceGetProcAddress,
    p_library: GDExtensionClassLibraryPtr,
    r_initialization: *mut GDExtensionInitialization,
) -> GDExtensionBool;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionGodotVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub string: *const std::ffi::c_char,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionGodotVersion2 {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub hex: u32,
    pub status: *const std::ffi::c_char,
    pub build: *const std::ffi::c_char,
    pub hash: *const std::ffi::c_char,
    pub timestamp: u64,
    pub string: *const std::ffi::c_char,
}
pub type GDExtensionMainLoopStartupCallback = unsafe extern "C" fn() -> ();
pub type GDExtensionMainLoopShutdownCallback = unsafe extern "C" fn() -> ();
pub type GDExtensionMainLoopFrameCallback = unsafe extern "C" fn() -> ();
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionMainLoopCallbacks {
    pub startup_func: GDExtensionMainLoopStartupCallback,
    pub shutdown_func: GDExtensionMainLoopShutdownCallback,
    pub frame_func: GDExtensionMainLoopFrameCallback,
}
pub struct GDExtensionInterface {
    #[doc = "Gets the Godot version that the GDExtension was loaded into."]
    pub get_godot_version:
        unsafe extern "C" fn(r_godot_version: *mut GDExtensionGodotVersion) -> (),
    #[doc = "Gets the Godot version that the GDExtension was loaded into."]
    pub get_godot_version2:
        unsafe extern "C" fn(r_godot_version: *mut GDExtensionGodotVersion2) -> (),
    #[doc = "Allocates memory."]
    pub mem_alloc: unsafe extern "C" fn(p_bytes: usize) -> *mut (),
    #[doc = "Reallocates memory."]
    pub mem_realloc: unsafe extern "C" fn(p_ptr: *mut (), p_bytes: usize) -> *mut (),
    #[doc = "Frees memory."]
    pub mem_free: unsafe extern "C" fn(p_ptr: *mut ()) -> (),
    #[doc = "Allocates memory."]
    pub mem_alloc2: unsafe extern "C" fn(p_bytes: usize, p_pad_align: GDExtensionBool) -> *mut (),
    #[doc = "Reallocates memory."]
    pub mem_realloc2: unsafe extern "C" fn(
        p_ptr: *mut (),
        p_bytes: usize,
        p_pad_align: GDExtensionBool,
    ) -> *mut (),
    #[doc = "Frees memory."]
    pub mem_free2: unsafe extern "C" fn(p_ptr: *mut (), p_pad_align: GDExtensionBool) -> (),
    #[doc = "Logs an error to Godot's built-in debugger and to the OS terminal."]
    pub print_error: unsafe extern "C" fn(
        p_description: *const std::ffi::c_char,
        p_function: *const std::ffi::c_char,
        p_file: *const std::ffi::c_char,
        p_line: i32,
        p_editor_notify: GDExtensionBool,
    ) -> (),
    #[doc = "Logs an error with a message to Godot's built-in debugger and to the OS terminal."]
    pub print_error_with_message: unsafe extern "C" fn(
        p_description: *const std::ffi::c_char,
        p_message: *const std::ffi::c_char,
        p_function: *const std::ffi::c_char,
        p_file: *const std::ffi::c_char,
        p_line: i32,
        p_editor_notify: GDExtensionBool,
    ) -> (),
    #[doc = "Logs a warning to Godot's built-in debugger and to the OS terminal."]
    pub print_warning: unsafe extern "C" fn(
        p_description: *const std::ffi::c_char,
        p_function: *const std::ffi::c_char,
        p_file: *const std::ffi::c_char,
        p_line: i32,
        p_editor_notify: GDExtensionBool,
    ) -> (),
    #[doc = "Logs a warning with a message to Godot's built-in debugger and to the OS terminal."]
    pub print_warning_with_message: unsafe extern "C" fn(
        p_description: *const std::ffi::c_char,
        p_message: *const std::ffi::c_char,
        p_function: *const std::ffi::c_char,
        p_file: *const std::ffi::c_char,
        p_line: i32,
        p_editor_notify: GDExtensionBool,
    ) -> (),
    #[doc = "Logs a script error to Godot's built-in debugger and to the OS terminal."]
    pub print_script_error: unsafe extern "C" fn(
        p_description: *const std::ffi::c_char,
        p_function: *const std::ffi::c_char,
        p_file: *const std::ffi::c_char,
        p_line: i32,
        p_editor_notify: GDExtensionBool,
    ) -> (),
    #[doc = "Logs a script error with a message to Godot's built-in debugger and to the OS terminal."]
    pub print_script_error_with_message: unsafe extern "C" fn(
        p_description: *const std::ffi::c_char,
        p_message: *const std::ffi::c_char,
        p_function: *const std::ffi::c_char,
        p_file: *const std::ffi::c_char,
        p_line: i32,
        p_editor_notify: GDExtensionBool,
    ) -> (),
    #[doc = "Gets the size of a native struct (ex. ObjectID) in bytes."]
    pub get_native_struct_size: unsafe extern "C" fn(p_name: GDExtensionConstStringNamePtr) -> u64,
    #[doc = "Copies one Variant into a another."]
    pub variant_new_copy: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedVariantPtr,
        p_src: GDExtensionConstVariantPtr,
    ) -> (),
    #[doc = "Creates a new Variant containing nil."]
    pub variant_new_nil: unsafe extern "C" fn(r_dest: GDExtensionUninitializedVariantPtr) -> (),
    #[doc = "Destroys a Variant."]
    pub variant_destroy: unsafe extern "C" fn(p_self: GDExtensionVariantPtr) -> (),
    #[doc = "Calls a method on a Variant."]
    pub variant_call: unsafe extern "C" fn(
        p_self: GDExtensionVariantPtr,
        p_method: GDExtensionConstStringNamePtr,
        p_args: *const GDExtensionConstVariantPtr,
        p_argument_count: GDExtensionInt,
        r_return: GDExtensionUninitializedVariantPtr,
        r_error: *mut GDExtensionCallError,
    ) -> (),
    #[doc = "Calls a static method on a Variant."]
    pub variant_call_static: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_method: GDExtensionConstStringNamePtr,
        p_args: *const GDExtensionConstVariantPtr,
        p_argument_count: GDExtensionInt,
        r_return: GDExtensionUninitializedVariantPtr,
        r_error: *mut GDExtensionCallError,
    ) -> (),
    #[doc = "Evaluate an operator on two Variants."]
    pub variant_evaluate: unsafe extern "C" fn(
        p_op: GDExtensionVariantOperator,
        p_a: GDExtensionConstVariantPtr,
        p_b: GDExtensionConstVariantPtr,
        r_return: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Sets a key on a Variant to a value."]
    pub variant_set: unsafe extern "C" fn(
        p_self: GDExtensionVariantPtr,
        p_key: GDExtensionConstVariantPtr,
        p_value: GDExtensionConstVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Sets a named key on a Variant to a value."]
    pub variant_set_named: unsafe extern "C" fn(
        p_self: GDExtensionVariantPtr,
        p_key: GDExtensionConstStringNamePtr,
        p_value: GDExtensionConstVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Sets a keyed property on a Variant to a value."]
    pub variant_set_keyed: unsafe extern "C" fn(
        p_self: GDExtensionVariantPtr,
        p_key: GDExtensionConstVariantPtr,
        p_value: GDExtensionConstVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Sets an index on a Variant to a value."]
    pub variant_set_indexed: unsafe extern "C" fn(
        p_self: GDExtensionVariantPtr,
        p_index: GDExtensionInt,
        p_value: GDExtensionConstVariantPtr,
        r_valid: *mut GDExtensionBool,
        r_oob: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Gets the value of a key from a Variant."]
    pub variant_get: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_key: GDExtensionConstVariantPtr,
        r_ret: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Gets the value of a named key from a Variant."]
    pub variant_get_named: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_key: GDExtensionConstStringNamePtr,
        r_ret: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Gets the value of a keyed property from a Variant."]
    pub variant_get_keyed: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_key: GDExtensionConstVariantPtr,
        r_ret: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Gets the value of an index from a Variant."]
    pub variant_get_indexed: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_index: GDExtensionInt,
        r_ret: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
        r_oob: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Initializes an iterator over a Variant."]
    pub variant_iter_init: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        r_iter: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> GDExtensionBool,
    #[doc = "Gets the next value for an iterator over a Variant."]
    pub variant_iter_next: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        r_iter: GDExtensionVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> GDExtensionBool,
    #[doc = "Gets the next value for an iterator over a Variant."]
    pub variant_iter_get: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        r_iter: GDExtensionVariantPtr,
        r_ret: GDExtensionUninitializedVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> (),
    #[doc = "Gets the hash of a Variant."]
    pub variant_hash: unsafe extern "C" fn(p_self: GDExtensionConstVariantPtr) -> GDExtensionInt,
    #[doc = "Gets the recursive hash of a Variant."]
    pub variant_recursive_hash: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_recursion_count: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Compares two Variants by their hash."]
    pub variant_hash_compare: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_other: GDExtensionConstVariantPtr,
    ) -> GDExtensionBool,
    #[doc = "Converts a Variant to a boolean."]
    pub variant_booleanize:
        unsafe extern "C" fn(p_self: GDExtensionConstVariantPtr) -> GDExtensionBool,
    #[doc = "Duplicates a Variant."]
    pub variant_duplicate: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        r_ret: GDExtensionVariantPtr,
        p_deep: GDExtensionBool,
    ) -> (),
    #[doc = "Converts a Variant to a string."]
    pub variant_stringify:
        unsafe extern "C" fn(p_self: GDExtensionConstVariantPtr, r_ret: GDExtensionStringPtr) -> (),
    #[doc = "Gets the type of a Variant."]
    pub variant_get_type:
        unsafe extern "C" fn(p_self: GDExtensionConstVariantPtr) -> GDExtensionVariantType,
    #[doc = "Checks if a Variant has the given method."]
    pub variant_has_method: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_method: GDExtensionConstStringNamePtr,
    ) -> GDExtensionBool,
    #[doc = "Checks if a type of Variant has the given member."]
    pub variant_has_member: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_member: GDExtensionConstStringNamePtr,
    ) -> GDExtensionBool,
    #[doc = "Checks if a Variant has a key."]
    pub variant_has_key: unsafe extern "C" fn(
        p_self: GDExtensionConstVariantPtr,
        p_key: GDExtensionConstVariantPtr,
        r_valid: *mut GDExtensionBool,
    ) -> GDExtensionBool,
    #[doc = "Gets the object instance ID from a variant of type GDEXTENSION_VARIANT_TYPE_OBJECT.\nIf the variant isn't of type GDEXTENSION_VARIANT_TYPE_OBJECT, then zero will be returned.\nThe instance ID will be returned even if the object is no longer valid - use `object_get_instance_by_id()` to check if the object is still valid."]
    pub variant_get_object_instance_id:
        unsafe extern "C" fn(p_self: GDExtensionConstVariantPtr) -> GDObjectInstanceID,
    #[doc = "Gets the name of a Variant type."]
    pub variant_get_type_name: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        r_name: GDExtensionUninitializedStringPtr,
    ) -> (),
    #[doc = "Checks if Variants can be converted from one type to another."]
    pub variant_can_convert: unsafe extern "C" fn(
        p_from: GDExtensionVariantType,
        p_to: GDExtensionVariantType,
    ) -> GDExtensionBool,
    #[doc = "Checks if Variant can be converted from one type to another using stricter rules."]
    pub variant_can_convert_strict: unsafe extern "C" fn(
        p_from: GDExtensionVariantType,
        p_to: GDExtensionVariantType,
    ) -> GDExtensionBool,
    #[doc = "Gets a pointer to a function that can create a Variant of the given type from a raw value."]
    pub get_variant_from_type_constructor:
        unsafe extern "C" fn(
            p_type: GDExtensionVariantType,
        ) -> GDExtensionVariantFromTypeConstructorFunc,
    #[doc = "Gets a pointer to a function that can get the raw value from a Variant of the given type."]
    pub get_variant_to_type_constructor:
        unsafe extern "C" fn(
            p_type: GDExtensionVariantType,
        ) -> GDExtensionTypeFromVariantConstructorFunc,
    #[doc = "Provides a function pointer for retrieving a pointer to a variant's internal value.\nAccess to a variant's internal value can be used to modify it in-place, or to retrieve its value without the overhead of variant conversion functions.\nIt is recommended to cache the getter for all variant types in a function table to avoid retrieval overhead upon use.\n\nEach function assumes the variant's type has already been determined and matches the function.\nInvoking the function with a variant of a mismatched type has undefined behavior, and may lead to a segmentation fault."]
    pub variant_get_ptr_internal_getter:
        unsafe extern "C" fn(
            p_type: GDExtensionVariantType,
        ) -> GDExtensionVariantGetInternalPtrFunc,
    #[doc = "Gets a pointer to a function that can evaluate the given Variant operator on the given Variant types."]
    pub variant_get_ptr_operator_evaluator: unsafe extern "C" fn(
        p_operator: GDExtensionVariantOperator,
        p_type_a: GDExtensionVariantType,
        p_type_b: GDExtensionVariantType,
    )
        -> GDExtensionPtrOperatorEvaluator,
    #[doc = "Gets a pointer to a function that can call a builtin method on a type of Variant."]
    pub variant_get_ptr_builtin_method: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_method: GDExtensionConstStringNamePtr,
        p_hash: GDExtensionInt,
    ) -> GDExtensionPtrBuiltInMethod,
    #[doc = "Gets a pointer to a function that can call one of the constructors for a type of Variant."]
    pub variant_get_ptr_constructor: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_constructor: i32,
    ) -> GDExtensionPtrConstructor,
    #[doc = "Gets a pointer to a function than can call the destructor for a type of Variant."]
    pub variant_get_ptr_destructor:
        unsafe extern "C" fn(p_type: GDExtensionVariantType) -> GDExtensionPtrDestructor,
    #[doc = "Constructs a Variant of the given type, using the first constructor that matches the given arguments."]
    pub variant_construct: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        r_base: GDExtensionUninitializedVariantPtr,
        p_args: *const GDExtensionConstVariantPtr,
        p_argument_count: i32,
        r_error: *mut GDExtensionCallError,
    ) -> (),
    #[doc = "Gets a pointer to a function that can call a member's setter on the given Variant type."]
    pub variant_get_ptr_setter: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_member: GDExtensionConstStringNamePtr,
    ) -> GDExtensionPtrSetter,
    #[doc = "Gets a pointer to a function that can call a member's getter on the given Variant type."]
    pub variant_get_ptr_getter: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_member: GDExtensionConstStringNamePtr,
    ) -> GDExtensionPtrGetter,
    #[doc = "Gets a pointer to a function that can set an index on the given Variant type."]
    pub variant_get_ptr_indexed_setter:
        unsafe extern "C" fn(p_type: GDExtensionVariantType) -> GDExtensionPtrIndexedSetter,
    #[doc = "Gets a pointer to a function that can get an index on the given Variant type."]
    pub variant_get_ptr_indexed_getter:
        unsafe extern "C" fn(p_type: GDExtensionVariantType) -> GDExtensionPtrIndexedGetter,
    #[doc = "Gets a pointer to a function that can set a key on the given Variant type."]
    pub variant_get_ptr_keyed_setter:
        unsafe extern "C" fn(p_type: GDExtensionVariantType) -> GDExtensionPtrKeyedSetter,
    #[doc = "Gets a pointer to a function that can get a key on the given Variant type."]
    pub variant_get_ptr_keyed_getter:
        unsafe extern "C" fn(p_type: GDExtensionVariantType) -> GDExtensionPtrKeyedGetter,
    #[doc = "Gets a pointer to a function that can check a key on the given Variant type."]
    pub variant_get_ptr_keyed_checker:
        unsafe extern "C" fn(p_type: GDExtensionVariantType) -> GDExtensionPtrKeyedChecker,
    #[doc = "Gets the value of a constant from the given Variant type."]
    pub variant_get_constant_value: unsafe extern "C" fn(
        p_type: GDExtensionVariantType,
        p_constant: GDExtensionConstStringNamePtr,
        r_ret: GDExtensionUninitializedVariantPtr,
    ) -> (),
    #[doc = "Gets a pointer to a function that can call a Variant utility function."]
    pub variant_get_ptr_utility_function: unsafe extern "C" fn(
        p_function: GDExtensionConstStringNamePtr,
        p_hash: GDExtensionInt,
    )
        -> GDExtensionPtrUtilityFunction,
    #[doc = "Creates a String from a Latin-1 encoded C string."]
    pub string_new_with_latin1_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const std::ffi::c_char,
    ) -> (),
    #[doc = "Creates a String from a UTF-8 encoded C string."]
    pub string_new_with_utf8_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const std::ffi::c_char,
    ) -> (),
    #[doc = "Creates a String from a UTF-16 encoded C string."]
    pub string_new_with_utf16_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const char16_t,
    ) -> (),
    #[doc = "Creates a String from a UTF-32 encoded C string."]
    pub string_new_with_utf32_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const char32_t,
    ) -> (),
    #[doc = "Creates a String from a wide C string."]
    pub string_new_with_wide_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const wchar_t,
    ) -> (),
    #[doc = "Creates a String from a Latin-1 encoded C string with the given length."]
    pub string_new_with_latin1_chars_and_len: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const std::ffi::c_char,
        p_size: GDExtensionInt,
    ) -> (),
    #[doc = "Creates a String from a UTF-8 encoded C string with the given length."]
    pub string_new_with_utf8_chars_and_len: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const std::ffi::c_char,
        p_size: GDExtensionInt,
    ) -> (),
    #[doc = "Creates a String from a UTF-8 encoded C string with the given length."]
    pub string_new_with_utf8_chars_and_len2: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const std::ffi::c_char,
        p_size: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Creates a String from a UTF-16 encoded C string with the given length."]
    pub string_new_with_utf16_chars_and_len: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const char16_t,
        p_char_count: GDExtensionInt,
    ) -> (),
    #[doc = "Creates a String from a UTF-16 encoded C string with the given length."]
    pub string_new_with_utf16_chars_and_len2: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const char16_t,
        p_char_count: GDExtensionInt,
        p_default_little_endian: GDExtensionBool,
    ) -> GDExtensionInt,
    #[doc = "Creates a String from a UTF-32 encoded C string with the given length."]
    pub string_new_with_utf32_chars_and_len: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const char32_t,
        p_char_count: GDExtensionInt,
    ) -> (),
    #[doc = "Creates a String from a wide C string with the given length."]
    pub string_new_with_wide_chars_and_len: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringPtr,
        p_contents: *const wchar_t,
        p_char_count: GDExtensionInt,
    ) -> (),
    #[doc = "Converts a String to a Latin-1 encoded C string.\nIt doesn't write a null terminator."]
    pub string_to_latin1_chars: unsafe extern "C" fn(
        p_self: GDExtensionConstStringPtr,
        r_text: *mut std::ffi::c_char,
        p_max_write_length: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Converts a String to a UTF-8 encoded C string.\nIt doesn't write a null terminator."]
    pub string_to_utf8_chars: unsafe extern "C" fn(
        p_self: GDExtensionConstStringPtr,
        r_text: *mut std::ffi::c_char,
        p_max_write_length: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Converts a String to a UTF-16 encoded C string.\nIt doesn't write a null terminator."]
    pub string_to_utf16_chars: unsafe extern "C" fn(
        p_self: GDExtensionConstStringPtr,
        r_text: *mut char16_t,
        p_max_write_length: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Converts a String to a UTF-32 encoded C string.\nIt doesn't write a null terminator."]
    pub string_to_utf32_chars: unsafe extern "C" fn(
        p_self: GDExtensionConstStringPtr,
        r_text: *mut char32_t,
        p_max_write_length: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Converts a String to a wide C string.\nIt doesn't write a null terminator."]
    pub string_to_wide_chars: unsafe extern "C" fn(
        p_self: GDExtensionConstStringPtr,
        r_text: *mut wchar_t,
        p_max_write_length: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Gets a pointer to the character at the given index from a String."]
    pub string_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionStringPtr,
        p_index: GDExtensionInt,
    ) -> *mut char32_t,
    #[doc = "Gets a const pointer to the character at the given index from a String."]
    pub string_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstStringPtr,
        p_index: GDExtensionInt,
    ) -> *const char32_t,
    #[doc = "Appends another String to a String."]
    pub string_operator_plus_eq_string:
        unsafe extern "C" fn(p_self: GDExtensionStringPtr, p_b: GDExtensionConstStringPtr) -> (),
    #[doc = "Appends a character to a String."]
    pub string_operator_plus_eq_char:
        unsafe extern "C" fn(p_self: GDExtensionStringPtr, p_b: char32_t) -> (),
    #[doc = "Appends a Latin-1 encoded C string to a String."]
    pub string_operator_plus_eq_cstr:
        unsafe extern "C" fn(p_self: GDExtensionStringPtr, p_b: *const std::ffi::c_char) -> (),
    #[doc = "Appends a wide C string to a String."]
    pub string_operator_plus_eq_wcstr:
        unsafe extern "C" fn(p_self: GDExtensionStringPtr, p_b: *const wchar_t) -> (),
    #[doc = "Appends a UTF-32 encoded C string to a String."]
    pub string_operator_plus_eq_c32str:
        unsafe extern "C" fn(p_self: GDExtensionStringPtr, p_b: *const char32_t) -> (),
    #[doc = "Resizes the underlying string data to the given number of characters.\nSpace needs to be allocated for the null terminating character ('\\0') which\nalso must be added manually, in order for all string functions to work correctly.\n\nWarning: This is an error-prone operation - only use it if there's no other\nefficient way to accomplish your goal."]
    pub string_resize: unsafe extern "C" fn(
        p_self: GDExtensionStringPtr,
        p_resize: GDExtensionInt,
    ) -> GDExtensionInt,
    #[doc = "Creates a StringName from a Latin-1 encoded C string.\nIf `p_is_static` is true, then:\n- The StringName will reuse the `p_contents` buffer instead of copying it.\n- You must guarantee that the buffer remains valid for the duration of the application (e.g. string literal).\n- You must not call a destructor for this StringName. Incrementing the initial reference once should achieve this.\n\n`p_is_static` is purely an optimization and can easily introduce undefined behavior if used wrong. In case of doubt, set it to false."]
    pub string_name_new_with_latin1_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringNamePtr,
        p_contents: *const std::ffi::c_char,
        p_is_static: GDExtensionBool,
    ) -> (),
    #[doc = "Creates a StringName from a UTF-8 encoded C string."]
    pub string_name_new_with_utf8_chars: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringNamePtr,
        p_contents: *const std::ffi::c_char,
    ) -> (),
    #[doc = "Creates a StringName from a UTF-8 encoded string with a given number of characters."]
    pub string_name_new_with_utf8_chars_and_len: unsafe extern "C" fn(
        r_dest: GDExtensionUninitializedStringNamePtr,
        p_contents: *const std::ffi::c_char,
        p_size: GDExtensionInt,
    ) -> (),
    #[doc = "Opens a raw XML buffer on an XMLParser instance."]
    pub xml_parser_open_buffer: unsafe extern "C" fn(
        p_instance: GDExtensionObjectPtr,
        p_buffer: *const u8,
        p_size: usize,
    ) -> GDExtensionInt,
    #[doc = "Stores the given buffer using an instance of FileAccess."]
    pub file_access_store_buffer: unsafe extern "C" fn(
        p_instance: GDExtensionObjectPtr,
        p_src: *const u8,
        p_length: u64,
    ) -> (),
    #[doc = "Reads the next p_length bytes into the given buffer using an instance of FileAccess."]
    pub file_access_get_buffer: unsafe extern "C" fn(
        p_instance: GDExtensionConstObjectPtr,
        p_dst: *mut u8,
        p_length: u64,
    ) -> u64,
    #[doc = "Returns writable pointer to internal Image buffer."]
    pub image_ptrw: unsafe extern "C" fn(p_instance: GDExtensionObjectPtr) -> *mut u8,
    #[doc = "Returns read only pointer to internal Image buffer."]
    pub image_ptr: unsafe extern "C" fn(p_instance: GDExtensionObjectPtr) -> *const u8,
    #[doc = "Adds a group task to an instance of WorkerThreadPool."]
    pub worker_thread_pool_add_native_group_task: unsafe extern "C" fn(
        p_instance: GDExtensionObjectPtr,
        p_func: GDExtensionWorkerThreadPoolGroupTask,
        p_userdata: *mut (),
        p_elements: std::ffi::c_int,
        p_tasks: std::ffi::c_int,
        p_high_priority: GDExtensionBool,
        p_description: GDExtensionConstStringPtr,
    ) -> i64,
    #[doc = "Adds a task to an instance of WorkerThreadPool."]
    pub worker_thread_pool_add_native_task: unsafe extern "C" fn(
        p_instance: GDExtensionObjectPtr,
        p_func: GDExtensionWorkerThreadPoolTask,
        p_userdata: *mut (),
        p_high_priority: GDExtensionBool,
        p_description: GDExtensionConstStringPtr,
    ) -> i64,
    #[doc = "Gets a pointer to a byte in a PackedByteArray."]
    pub packed_byte_array_operator_index:
        unsafe extern "C" fn(p_self: GDExtensionTypePtr, p_index: GDExtensionInt) -> *mut u8,
    #[doc = "Gets a const pointer to a byte in a PackedByteArray."]
    pub packed_byte_array_operator_index_const:
        unsafe extern "C" fn(p_self: GDExtensionConstTypePtr, p_index: GDExtensionInt) -> *const u8,
    #[doc = "Gets a pointer to a 32-bit float in a PackedFloat32Array."]
    pub packed_float32_array_operator_index:
        unsafe extern "C" fn(p_self: GDExtensionTypePtr, p_index: GDExtensionInt) -> *mut f32,
    #[doc = "Gets a const pointer to a 32-bit float in a PackedFloat32Array."]
    pub packed_float32_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> *const f32,
    #[doc = "Gets a pointer to a 64-bit float in a PackedFloat64Array."]
    pub packed_float64_array_operator_index:
        unsafe extern "C" fn(p_self: GDExtensionTypePtr, p_index: GDExtensionInt) -> *mut f64,
    #[doc = "Gets a const pointer to a 64-bit float in a PackedFloat64Array."]
    pub packed_float64_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> *const f64,
    #[doc = "Gets a pointer to a 32-bit integer in a PackedInt32Array."]
    pub packed_int32_array_operator_index:
        unsafe extern "C" fn(p_self: GDExtensionTypePtr, p_index: GDExtensionInt) -> *mut i32,
    #[doc = "Gets a const pointer to a 32-bit integer in a PackedInt32Array."]
    pub packed_int32_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> *const i32,
    #[doc = "Gets a pointer to a 64-bit integer in a PackedInt64Array."]
    pub packed_int64_array_operator_index:
        unsafe extern "C" fn(p_self: GDExtensionTypePtr, p_index: GDExtensionInt) -> *mut i64,
    #[doc = "Gets a const pointer to a 64-bit integer in a PackedInt64Array."]
    pub packed_int64_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> *const i64,
    #[doc = "Gets a pointer to a string in a PackedStringArray."]
    pub packed_string_array_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionStringPtr,
    #[doc = "Gets a const pointer to a string in a PackedStringArray."]
    pub packed_string_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionStringPtr,
    #[doc = "Gets a pointer to a Vector2 in a PackedVector2Array."]
    pub packed_vector2_array_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a const pointer to a Vector2 in a PackedVector2Array."]
    pub packed_vector2_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a pointer to a Vector3 in a PackedVector3Array."]
    pub packed_vector3_array_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a const pointer to a Vector3 in a PackedVector3Array."]
    pub packed_vector3_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a pointer to a Vector4 in a PackedVector4Array."]
    pub packed_vector4_array_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a const pointer to a Vector4 in a PackedVector4Array."]
    pub packed_vector4_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a pointer to a color in a PackedColorArray."]
    pub packed_color_array_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a const pointer to a color in a PackedColorArray."]
    pub packed_color_array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionTypePtr,
    #[doc = "Gets a pointer to a Variant in an Array."]
    pub array_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionVariantPtr,
    #[doc = "Gets a const pointer to a Variant in an Array."]
    pub array_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_index: GDExtensionInt,
    ) -> GDExtensionVariantPtr,
    #[doc = "Sets an Array to be a reference to another Array object."]
    pub array_ref:
        unsafe extern "C" fn(p_self: GDExtensionTypePtr, p_from: GDExtensionConstTypePtr) -> (),
    #[doc = "Makes an Array into a typed Array."]
    pub array_set_typed: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_type: GDExtensionVariantType,
        p_class_name: GDExtensionConstStringNamePtr,
        p_script: GDExtensionConstVariantPtr,
    ) -> (),
    #[doc = "Gets a pointer to a Variant in a Dictionary with the given key."]
    pub dictionary_operator_index: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_key: GDExtensionConstVariantPtr,
    ) -> GDExtensionVariantPtr,
    #[doc = "Gets a const pointer to a Variant in a Dictionary with the given key."]
    pub dictionary_operator_index_const: unsafe extern "C" fn(
        p_self: GDExtensionConstTypePtr,
        p_key: GDExtensionConstVariantPtr,
    ) -> GDExtensionVariantPtr,
    #[doc = "Makes a Dictionary into a typed Dictionary."]
    pub dictionary_set_typed: unsafe extern "C" fn(
        p_self: GDExtensionTypePtr,
        p_key_type: GDExtensionVariantType,
        p_key_class_name: GDExtensionConstStringNamePtr,
        p_key_script: GDExtensionConstVariantPtr,
        p_value_type: GDExtensionVariantType,
        p_value_class_name: GDExtensionConstStringNamePtr,
        p_value_script: GDExtensionConstVariantPtr,
    ) -> (),
    #[doc = "Calls a method on an Object."]
    pub object_method_bind_call: unsafe extern "C" fn(
        p_method_bind: GDExtensionMethodBindPtr,
        p_instance: GDExtensionObjectPtr,
        p_args: *const GDExtensionConstVariantPtr,
        p_arg_count: GDExtensionInt,
        r_ret: GDExtensionUninitializedVariantPtr,
        r_error: *mut GDExtensionCallError,
    ) -> (),
    #[doc = "Calls a method on an Object (using a \"ptrcall\")."]
    pub object_method_bind_ptrcall: unsafe extern "C" fn(
        p_method_bind: GDExtensionMethodBindPtr,
        p_instance: GDExtensionObjectPtr,
        p_args: *const GDExtensionConstTypePtr,
        r_ret: GDExtensionTypePtr,
    ) -> (),
    #[doc = "Destroys an Object."]
    pub object_destroy: unsafe extern "C" fn(p_o: GDExtensionObjectPtr) -> (),
    #[doc = "Gets a global singleton by name."]
    pub global_get_singleton:
        unsafe extern "C" fn(p_name: GDExtensionConstStringNamePtr) -> GDExtensionObjectPtr,
    #[doc = "Gets a pointer representing an Object's instance binding."]
    pub object_get_instance_binding: unsafe extern "C" fn(
        p_o: GDExtensionObjectPtr,
        p_token: *mut (),
        p_callbacks: *const GDExtensionInstanceBindingCallbacks,
    ) -> *mut (),
    #[doc = "Sets an Object's instance binding."]
    pub object_set_instance_binding: unsafe extern "C" fn(
        p_o: GDExtensionObjectPtr,
        p_token: *mut (),
        p_binding: *mut (),
        p_callbacks: *const GDExtensionInstanceBindingCallbacks,
    ) -> (),
    #[doc = "Free an Object's instance binding."]
    pub object_free_instance_binding:
        unsafe extern "C" fn(p_o: GDExtensionObjectPtr, p_token: *mut ()) -> (),
    #[doc = "Sets an extension class instance on a Object.\n`p_classname` should be a registered extension class and should extend the `p_o` Object's class."]
    pub object_set_instance: unsafe extern "C" fn(
        p_o: GDExtensionObjectPtr,
        p_classname: GDExtensionConstStringNamePtr,
        p_instance: GDExtensionClassInstancePtr,
    ) -> (),
    #[doc = "Gets the class name of an Object.\nIf the GDExtension wraps the Godot object in an abstraction specific to its class, this is the\nfunction that should be used to determine which wrapper to use."]
    pub object_get_class_name: unsafe extern "C" fn(
        p_object: GDExtensionConstObjectPtr,
        p_library: GDExtensionClassLibraryPtr,
        r_class_name: GDExtensionUninitializedStringNamePtr,
    ) -> GDExtensionBool,
    #[doc = "Casts an Object to a different type."]
    pub object_cast_to: unsafe extern "C" fn(
        p_object: GDExtensionConstObjectPtr,
        p_class_tag: *mut (),
    ) -> GDExtensionObjectPtr,
    #[doc = "Gets an Object by its instance ID."]
    pub object_get_instance_from_id:
        unsafe extern "C" fn(p_instance_id: GDObjectInstanceID) -> GDExtensionObjectPtr,
    #[doc = "Gets the instance ID from an Object."]
    pub object_get_instance_id:
        unsafe extern "C" fn(p_object: GDExtensionConstObjectPtr) -> GDObjectInstanceID,
    #[doc = "Checks if this object has a script with the given method."]
    pub object_has_script_method: unsafe extern "C" fn(
        p_object: GDExtensionConstObjectPtr,
        p_method: GDExtensionConstStringNamePtr,
    ) -> GDExtensionBool,
    #[doc = "Call the given script method on this object."]
    pub object_call_script_method: unsafe extern "C" fn(
        p_object: GDExtensionObjectPtr,
        p_method: GDExtensionConstStringNamePtr,
        p_args: *const GDExtensionConstVariantPtr,
        p_argument_count: GDExtensionInt,
        r_return: GDExtensionUninitializedVariantPtr,
        r_error: *mut GDExtensionCallError,
    ) -> (),
    #[doc = "Gets the Object from a reference."]
    pub ref_get_object: unsafe extern "C" fn(p_ref: GDExtensionConstRefPtr) -> GDExtensionObjectPtr,
    #[doc = "Sets the Object referred to by a reference."]
    pub ref_set_object:
        unsafe extern "C" fn(p_ref: GDExtensionRefPtr, p_object: GDExtensionObjectPtr) -> (),
    #[doc = "Creates a script instance that contains the given info and instance data."]
    pub script_instance_create: unsafe extern "C" fn(
        p_info: *const GDExtensionScriptInstanceInfo,
        p_instance_data: GDExtensionScriptInstanceDataPtr,
    ) -> GDExtensionScriptInstancePtr,
    #[doc = "Creates a script instance that contains the given info and instance data."]
    pub script_instance_create2: unsafe extern "C" fn(
        p_info: *const GDExtensionScriptInstanceInfo2,
        p_instance_data: GDExtensionScriptInstanceDataPtr,
    ) -> GDExtensionScriptInstancePtr,
    #[doc = "Creates a script instance that contains the given info and instance data."]
    pub script_instance_create3: unsafe extern "C" fn(
        p_info: *const GDExtensionScriptInstanceInfo3,
        p_instance_data: GDExtensionScriptInstanceDataPtr,
    ) -> GDExtensionScriptInstancePtr,
    #[doc = "Creates a placeholder script instance for a given script and instance.\nThis interface is optional as a custom placeholder could also be created with script_instance_create()."]
    pub placeholder_script_instance_create: unsafe extern "C" fn(
        p_language: GDExtensionObjectPtr,
        p_script: GDExtensionObjectPtr,
        p_owner: GDExtensionObjectPtr,
    )
        -> GDExtensionScriptInstancePtr,
    #[doc = "Updates a placeholder script instance with the given properties and values.\nThe passed in placeholder must be an instance of PlaceHolderScriptInstance\nsuch as the one returned by placeholder_script_instance_create()."]
    pub placeholder_script_instance_update: unsafe extern "C" fn(
        p_placeholder: GDExtensionScriptInstancePtr,
        p_properties: GDExtensionConstTypePtr,
        p_values: GDExtensionConstTypePtr,
    ) -> (),
    #[doc = "Get the script instance data attached to this object."]
    pub object_get_script_instance: unsafe extern "C" fn(
        p_object: GDExtensionConstObjectPtr,
        p_language: GDExtensionObjectPtr,
    ) -> GDExtensionScriptInstanceDataPtr,
    #[doc = "Set the script instance data attached to this object."]
    pub object_set_script_instance: unsafe extern "C" fn(
        p_object: GDExtensionObjectPtr,
        p_script_instance: GDExtensionScriptInstanceDataPtr,
    ) -> (),
    #[doc = "Creates a custom Callable object from a function pointer.\nProvided struct can be safely freed once the function returns."]
    pub callable_custom_create: unsafe extern "C" fn(
        r_callable: GDExtensionUninitializedTypePtr,
        p_callable_custom_info: *mut GDExtensionCallableCustomInfo,
    ) -> (),
    #[doc = "Creates a custom Callable object from a function pointer.\nProvided struct can be safely freed once the function returns."]
    pub callable_custom_create2: unsafe extern "C" fn(
        r_callable: GDExtensionUninitializedTypePtr,
        p_callable_custom_info: *mut GDExtensionCallableCustomInfo2,
    ) -> (),
    #[doc = "Retrieves the userdata pointer from a custom Callable.\nIf the Callable is not a custom Callable or the token does not match the one provided to callable_custom_create() via GDExtensionCallableCustomInfo then NULL will be returned."]
    pub callable_custom_get_userdata:
        unsafe extern "C" fn(p_callable: GDExtensionConstTypePtr, p_token: *mut ()) -> *mut (),
    #[doc = "Constructs an Object of the requested class.\nThe passed class must be a built-in godot class, or an already-registered extension class. In both cases, object_set_instance() should be called to fully initialize the object."]
    pub classdb_construct_object:
        unsafe extern "C" fn(p_classname: GDExtensionConstStringNamePtr) -> GDExtensionObjectPtr,
    #[doc = "Constructs an Object of the requested class.\nThe passed class must be a built-in godot class, or an already-registered extension class. In both cases, object_set_instance() should be called to fully initialize the object.\n\n\"NOTIFICATION_POSTINITIALIZE\" must be sent after construction."]
    pub classdb_construct_object2:
        unsafe extern "C" fn(p_classname: GDExtensionConstStringNamePtr) -> GDExtensionObjectPtr,
    #[doc = "Gets a pointer to the MethodBind in ClassDB for the given class, method and hash."]
    pub classdb_get_method_bind: unsafe extern "C" fn(
        p_classname: GDExtensionConstStringNamePtr,
        p_methodname: GDExtensionConstStringNamePtr,
        p_hash: GDExtensionInt,
    ) -> GDExtensionMethodBindPtr,
    #[doc = "Gets a pointer uniquely identifying the given built-in class in the ClassDB."]
    pub classdb_get_class_tag:
        unsafe extern "C" fn(p_classname: GDExtensionConstStringNamePtr) -> *mut (),
    #[doc = "Registers an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_parent_class_name: GDExtensionConstStringNamePtr,
        p_extension_funcs: *const GDExtensionClassCreationInfo,
    ) -> (),
    #[doc = "Registers an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class2: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_parent_class_name: GDExtensionConstStringNamePtr,
        p_extension_funcs: *const GDExtensionClassCreationInfo2,
    ) -> (),
    #[doc = "Registers an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class3: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_parent_class_name: GDExtensionConstStringNamePtr,
        p_extension_funcs: *const GDExtensionClassCreationInfo3,
    ) -> (),
    #[doc = "Registers an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class4: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_parent_class_name: GDExtensionConstStringNamePtr,
        p_extension_funcs: *const GDExtensionClassCreationInfo4,
    ) -> (),
    #[doc = "Registers an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class5: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_parent_class_name: GDExtensionConstStringNamePtr,
        p_extension_funcs: *const GDExtensionClassCreationInfo5,
    ) -> (),
    #[doc = "Registers a method on an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class_method: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_method_info: *const GDExtensionClassMethodInfo,
    ) -> (),
    #[doc = "Registers a virtual method on an extension class in ClassDB, that can be implemented by scripts or other extensions.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class_virtual_method: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_method_info: *const GDExtensionClassVirtualMethodInfo,
    ) -> (),
    #[doc = "Registers an integer constant on an extension class in the ClassDB.\nNote about registering bitfield values (if p_is_bitfield is true): even though p_constant_value is signed, language bindings are\nadvised to treat bitfields as uint64_t, since this is generally clearer and can prevent mistakes like using -1 for setting all bits.\nLanguage APIs should thus provide an abstraction that registers bitfields (uint64_t) separately from regular constants (int64_t)."]
    pub classdb_register_extension_class_integer_constant: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_enum_name: GDExtensionConstStringNamePtr,
        p_constant_name: GDExtensionConstStringNamePtr,
        p_constant_value: GDExtensionInt,
        p_is_bitfield: GDExtensionBool,
    ) -> (),
    #[doc = "Registers a property on an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class_property: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_info: *const GDExtensionPropertyInfo,
        p_setter: GDExtensionConstStringNamePtr,
        p_getter: GDExtensionConstStringNamePtr,
    ) -> (),
    #[doc = "Registers an indexed property on an extension class in the ClassDB.\nProvided struct can be safely freed once the function returns."]
    pub classdb_register_extension_class_property_indexed: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_info: *const GDExtensionPropertyInfo,
        p_setter: GDExtensionConstStringNamePtr,
        p_getter: GDExtensionConstStringNamePtr,
        p_index: GDExtensionInt,
    ) -> (),
    #[doc = "Registers a property group on an extension class in the ClassDB."]
    pub classdb_register_extension_class_property_group: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_group_name: GDExtensionConstStringPtr,
        p_prefix: GDExtensionConstStringPtr,
    ) -> (),
    #[doc = "Registers a property subgroup on an extension class in the ClassDB."]
    pub classdb_register_extension_class_property_subgroup: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_subgroup_name: GDExtensionConstStringPtr,
        p_prefix: GDExtensionConstStringPtr,
    ) -> (),
    #[doc = "Registers a signal on an extension class in the ClassDB.\nProvided structs can be safely freed once the function returns."]
    pub classdb_register_extension_class_signal: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
        p_signal_name: GDExtensionConstStringNamePtr,
        p_argument_info: *const GDExtensionPropertyInfo,
        p_argument_count: GDExtensionInt,
    ) -> (),
    #[doc = "Unregisters an extension class in the ClassDB.\nUnregistering a parent class before a class that inherits it will result in failure. Inheritors must be unregistered first."]
    pub classdb_unregister_extension_class: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_class_name: GDExtensionConstStringNamePtr,
    ) -> (),
    #[doc = "Gets the path to the current GDExtension library."]
    pub get_library_path: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        r_path: GDExtensionUninitializedStringPtr,
    ) -> (),
    #[doc = "Adds an editor plugin.\nIt's safe to call during initialization."]
    pub editor_add_plugin: unsafe extern "C" fn(p_class_name: GDExtensionConstStringNamePtr) -> (),
    #[doc = "Removes an editor plugin."]
    pub editor_remove_plugin:
        unsafe extern "C" fn(p_class_name: GDExtensionConstStringNamePtr) -> (),
    #[doc = "Loads new XML-formatted documentation data in the editor.\nThe provided pointer can be immediately freed once the function returns."]
    pub editor_help_load_xml_from_utf8_chars:
        unsafe extern "C" fn(p_data: *const std::ffi::c_char) -> (),
    #[doc = "Loads new XML-formatted documentation data in the editor.\nThe provided pointer can be immediately freed once the function returns."]
    pub editor_help_load_xml_from_utf8_chars_and_len:
        unsafe extern "C" fn(p_data: *const std::ffi::c_char, p_size: GDExtensionInt) -> (),
    #[doc = "Registers a callback that Godot can call to get the list of all classes (from ClassDB) that may be used by the calling GDExtension.\nThis is used by the editor to generate a build profile (in \"Tools\" > \"Engine Compilation Configuration Editor...\" > \"Detect from project\"),\nin order to recompile Godot with only the classes used.\nIn the provided callback, the GDExtension should provide the list of classes that _may_ be used statically, thus the time of invocation shouldn't matter.\nIf a GDExtension doesn't register a callback, Godot will assume that it could be using any classes."]
    pub editor_register_get_classes_used_callback: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_callback: GDExtensionEditorGetClassesUsedCallback,
    ) -> (),
    #[doc = "Registers callbacks to be called at different phases of the main loop."]
    pub register_main_loop_callbacks: unsafe extern "C" fn(
        p_library: GDExtensionClassLibraryPtr,
        p_callbacks: *const GDExtensionMainLoopCallbacks,
    ) -> (),
}
