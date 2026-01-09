/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{Array, GString, VarDictionary, Variant, VariantType};
use godot::classes::{ClassDb, RefCounted, Resource};
use godot::global::Error;
use godot::meta::ToGodot as _;
use godot::obj::{Gd, NewGd, Singleton};

use crate::framework::{runs_release, suppress_godot_print};

/// Test entire `ClassDB` API, to ensure it can be used at all init levels.
///
/// Not itself an #[itest] but invoked as part of the init stage callbacks.
pub fn check_classdb_full_api() {
    let mut db = ClassDb::singleton();

    // ClassDB.get_class_list()
    let classes = db.get_class_list();
    assert!(classes.contains("Object"));
    assert!(classes.contains("RefCounted"));

    // ClassDB.get_inheriters_from_class()
    let subclasses = db.get_inheriters_from_class("Object");
    assert!(subclasses.contains("RefCounted"));

    // ClassDB.get_parent_class()
    assert_eq!(db.get_parent_class("RefCounted"), "Object");
    assert!(db.get_parent_class("Object").is_empty());

    // ClassDB.class_exists()
    assert!(db.class_exists("Object"));
    assert!(db.class_exists("RefCounted"));
    assert!(!db.class_exists("NonExistentClass12345"));

    // ClassDB.is_parent_class()
    assert!(db.is_parent_class("RefCounted", "Object"));
    assert!(!db.is_parent_class("Object", "RefCounted"));

    // ClassDB.can_instantiate()
    assert!(db.can_instantiate("Object"));
    assert!(!db.can_instantiate("Script"));

    // ClassDB.instantiate()
    let variant = db.instantiate("RefCounted");
    assert!(!variant.is_nil());
    let obj = variant.to::<Gd<RefCounted>>();
    assert!(obj.is_instance_valid());

    // ClassDB.is_class_enabled()
    assert!(db.is_class_enabled("Object"));
    assert!(db.is_class_enabled("RefCounted"));

    // ClassDB.class_has_signal()
    assert!(db.class_has_signal("Object", "script_changed"));
    assert!(db.class_has_signal("Object", "property_list_changed"));

    // ClassDB.class_get_signal()
    let signal_info = db.class_get_signal("Object", "script_changed");
    let name = signal_info.get("name").expect("name key should exist");
    assert_eq!(name.to::<GString>(), "script_changed");

    // ClassDB.class_get_signal_list()
    let signals = db.class_get_signal_list("Object");
    assert!(!signals.is_empty());
    assert!(has_dict_named(signals, "script_changed"));

    // ClassDB.class_get_property_list()
    let properties = db.class_get_property_list("Resource");
    assert!(has_dict_named(properties, "resource_scene_unique_id"));

    // ClassDB.class_get_property() and class_set_property()
    let obj = Resource::new_gd();
    let result = db.class_set_property(&obj, "script", &Variant::nil());
    assert_eq!(result, Error::ERR_UNAVAILABLE);

    let result = db.class_set_property(&obj, "resource_scene_unique_id", &123.to_variant());
    // Release templates skip type validation, see https://github.com/godotengine/godot/issues/86264.
    if !runs_release() {
        assert_eq!(result, Error::ERR_INVALID_DATA);
    }

    let result = db.class_set_property(&obj, "resource_scene_unique_id", &"uid123".to_variant());
    assert_eq!(result, Error::OK);

    let rid = db.class_get_property(&obj, "resource_scene_unique_id");
    assert_eq!(rid, "uid123".to_variant());

    // ClassDB.class_has_method()
    assert!(db.class_has_method("Object", "get_class"));
    assert!(db.class_has_method("Object", "set"));
    assert!(db.class_has_method("Object", "get"));
    assert!(!db.class_has_method("Object", "xyz"));
    let has_set_method = db
        .class_has_method_ex("Object", "set")
        .no_inheritance(true)
        .done();
    assert!(has_set_method);

    // ClassDB.class_get_method_list()
    let methods = db.class_get_method_list("Object");
    assert!(!methods.is_empty());
    assert!(has_dict_named(methods, "get_class"));

    // ClassDB.class_get_integer_constant_list()
    let constants = db.class_get_integer_constant_list("Object");
    assert!(constants.contains("NOTIFICATION_POSTINITIALIZE"));
    assert!(constants.contains("NOTIFICATION_PREDELETE"));

    // ClassDB.class_has_integer_constant()
    assert!(db.class_has_integer_constant("Object", "NOTIFICATION_POSTINITIALIZE"));
    assert!(db.class_has_integer_constant("Object", "NOTIFICATION_PREDELETE"));
    assert!(!db.class_has_integer_constant("Object", "NONEXISTENT_CONSTANT_XYZ"));

    // ClassDB.class_get_integer_constant()
    let value = db.class_get_integer_constant("Object", "NOTIFICATION_POSTINITIALIZE");
    assert_eq!(value, 0);
    let value = db.class_get_integer_constant("Object", "NOTIFICATION_PREDELETE");
    assert_eq!(value, 1);

    // ClassDB.class_has_enum()
    assert!(db.class_has_enum("Object", "ConnectFlags"));
    assert!(!db.class_has_enum("Object", "NonexistentEnum"));

    // ClassDB.class_get_enum_list()
    let enums = db.class_get_enum_list("Object");
    assert!(enums.contains("ConnectFlags"));

    // ClassDB.class_get_enum_constants()
    let constants = db.class_get_enum_constants("Object", "ConnectFlags");
    assert!(!constants.is_empty());

    // ClassDB.class_get_integer_constant_enum()
    let enum_name = db.class_get_integer_constant_enum("Object", "CONNECT_DEFERRED");
    assert_eq!(enum_name, "ConnectFlags");
    let enum_name = db.class_get_integer_constant_enum("Object", "NONEXISTENT_CONSTANT_XYZ");
    assert_eq!(enum_name, "");

    // Tests for Godot 4.3+ APIs.
    #[cfg(since_api = "4.3")]
    {
        // ClassDB.class_get_property_default_value()
        let default = db.class_get_property_default_value("Object", "script");
        assert_eq!(default.get_type(), VariantType::NIL);

        // ClassDB.class_get_method_argument_count()
        assert_eq!(db.class_get_method_argument_count("Object", "set"), 2);
        assert_eq!(db.class_get_method_argument_count("Object", "get"), 1);

        // ClassDB.is_class_enum_bitfield()
        assert!(!db.is_class_enum_bitfield("Object", "ConnectFlags")); // Not a real bitfield.
    }

    // Tests for Godot 4.4+ APIs.
    #[cfg(since_api = "4.4")]
    {
        use godot::classes::class_db::ApiType;

        // ClassDB.class_get_api_type() -- classes not yet loaded will behave like unknown ones, returning NONE.
        assert_eq!(db.class_get_api_type("Object"), ApiType::CORE);
        suppress_godot_print(|| {
            assert_eq!(db.class_get_api_type("StaticBody4D"), ApiType::NONE);
        });

        // ClassDB.class_get_property_getter(), class_get_property_setter()
        let getter = db.class_get_property_getter("Resource", "resource_path");
        let setter = db.class_get_property_setter("Resource", "resource_path");
        assert!(!getter.is_empty());
        assert!(!setter.is_empty());

        // ClassDB.class_call_static()
        let exists = db.class_call_static("FileAccess", "file_exists", &["a/path".to_variant()]);
        assert_eq!(exists.get_type(), VariantType::BOOL); // Not interested in actual result, but that the method can be called.
    }
}

/// Check if one of the dictionaries in the list has a specific name key/value.
// `list` consumes value to avoid accidental usage after.
fn has_dict_named(list: Array<VarDictionary>, name: &str) -> bool {
    list.iter_shared().any(|dict| {
        dict.get("name")
            .map(|v| v.to::<GString>() == name)
            .unwrap_or(false)
    })
}
