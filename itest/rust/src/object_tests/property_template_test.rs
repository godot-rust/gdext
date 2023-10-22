/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Testing that GDScript and rust produces the same property info for properties exported to Godot.

// We're using some weird formatting just for simplicity's sake.
#![allow(non_snake_case)]

use std::collections::HashMap;

use crate::framework::itest;
use godot::{engine::global::PropertyUsageFlags, prelude::*};

use crate::framework::TestContext;

#[derive(GodotClass)]
#[class(base = Node, init)]
struct PropertyTemplateRust {
    // Base types
    #[var]
    property_bool: bool,
    #[var]
    property_i64: i64,
    #[var]
    property_i32: i32,
    #[var]
    property_i16: i16,
    #[var]
    property_i8: i8,
    #[var]
    property_u32: u32,
    #[var]
    property_u16: u16,
    #[var]
    property_u8: u8,
    #[var]
    property_f64: f64,
    #[var]
    property_f32: f32,
    #[var]
    property_GodotString: GodotString,
    #[var]
    property_Vector2: Vector2,
    #[var]
    property_Vector2i: Vector2i,
    #[var]
    property_Rect2: Rect2,
    #[var]
    property_Rect2i: Rect2i,
    #[var]
    property_Vector3: Vector3,
    #[var]
    property_Vector3i: Vector3i,
    #[var]
    property_Transform2D: Transform2D,
    #[var]
    property_Vector4: Vector4,
    #[var]
    property_Vector4i: Vector4i,
    #[var]
    #[init(default = Plane::new(Vector3::new(1.0,0.0,0.0), 0.0))]
    property_Plane: Plane,
    #[var]
    property_Quaternion: Quaternion,
    #[var]
    property_Aabb: Aabb,
    #[var]
    property_Basis: Basis,
    #[var]
    property_Transform3D: Transform3D,
    #[var]
    property_Projection: Projection,
    #[var]
    property_Color: Color,
    #[var]
    property_StringName: StringName,
    #[var]
    property_NodePath: NodePath,
    #[var]
    #[init(default = Rid::Invalid)]
    property_Rid: Rid,
    #[var]
    property_Gd_Node: Option<Gd<Node>>,
    #[var]
    property_Gd_Resource: Option<Gd<Resource>>,
    #[var]
    #[init(default = Callable::invalid())]
    property_Callable: Callable,
    #[var]
    property_Dictionary: Dictionary,
    #[var]
    property_VariantArray: VariantArray,
    #[var]
    property_PackedByteArray: PackedByteArray,
    #[var]
    property_PackedInt32Array: PackedInt32Array,
    #[var]
    property_PackedInt64Array: PackedInt64Array,
    #[var]
    property_PackedFloat32Array: PackedFloat32Array,
    #[var]
    property_PackedFloat64Array: PackedFloat64Array,
    #[var]
    property_PackedStringArray: PackedStringArray,
    #[var]
    property_PackedVector2Array: PackedVector2Array,
    #[var]
    property_PackedVector3Array: PackedVector3Array,
    #[var]
    property_PackedColorArray: PackedColorArray,
    // Types nested in arrays
    #[var]
    property_array_bool: Array<bool>,
    #[var]
    property_array_i64: Array<i64>,
    #[var]
    property_array_i32: Array<i32>,
    #[var]
    property_array_i16: Array<i16>,
    #[var]
    property_array_i8: Array<i8>,
    #[var]
    property_array_u32: Array<u32>,
    #[var]
    property_array_u16: Array<u16>,
    #[var]
    property_array_u8: Array<u8>,
    #[var]
    property_array_f64: Array<f64>,
    #[var]
    property_array_f32: Array<f32>,
    #[var]
    property_array_GodotString: Array<GodotString>,
    #[var]
    property_array_Vector2: Array<Vector2>,
    #[var]
    property_array_Vector2i: Array<Vector2i>,
    #[var]
    property_array_Rect2: Array<Rect2>,
    #[var]
    property_array_Rect2i: Array<Rect2i>,
    #[var]
    property_array_Vector3: Array<Vector3>,
    #[var]
    property_array_Vector3i: Array<Vector3i>,
    #[var]
    property_array_Transform2D: Array<Transform2D>,
    #[var]
    property_array_Vector4: Array<Vector4>,
    #[var]
    property_array_Vector4i: Array<Vector4i>,
    #[var]
    property_array_Plane: Array<Plane>,
    #[var]
    property_array_Quaternion: Array<Quaternion>,
    #[var]
    property_array_Aabb: Array<Aabb>,
    #[var]
    property_array_Basis: Array<Basis>,
    #[var]
    property_array_Transform3D: Array<Transform3D>,
    #[var]
    property_array_Projection: Array<Projection>,
    #[var]
    property_array_Color: Array<Color>,
    #[var]
    property_array_StringName: Array<StringName>,
    #[var]
    property_array_NodePath: Array<NodePath>,
    #[var]
    property_array_Rid: Array<Rid>,
    #[var]
    property_array_Gd_Node: Array<Gd<Node>>,
    #[var]
    property_array_Gd_Resource: Array<Gd<Resource>>,
    #[var]
    property_array_Callable: Array<Callable>,
    #[var]
    property_array_Dictionary: Array<Dictionary>,
    #[var]
    property_array_VariantArray: Array<VariantArray>,
    #[var]
    property_array_PackedByteArray: Array<PackedByteArray>,
    #[var]
    property_array_PackedInt32Array: Array<PackedInt32Array>,
    #[var]
    property_array_PackedInt64Array: Array<PackedInt64Array>,
    #[var]
    property_array_PackedFloat32Array: Array<PackedFloat32Array>,
    #[var]
    property_array_PackedFloat64Array: Array<PackedFloat64Array>,
    #[var]
    property_array_PackedStringArray: Array<PackedStringArray>,
    #[var]
    property_array_PackedVector2Array: Array<PackedVector2Array>,
    #[var]
    property_array_PackedVector3Array: Array<PackedVector3Array>,
    #[var]
    property_array_PackedColorArray: Array<PackedColorArray>,

    // Exporting base types
    #[export]
    export_bool: bool,
    #[export]
    export_i64: i64,
    #[export]
    export_i32: i32,
    #[export]
    export_i16: i16,
    #[export]
    export_i8: i8,
    #[export]
    export_u32: u32,
    #[export]
    export_u16: u16,
    #[export]
    export_u8: u8,
    #[export]
    export_f64: f64,
    #[export]
    export_f32: f32,
    #[export]
    export_GodotString: GodotString,
    #[export]
    export_Vector2: Vector2,
    #[export]
    export_Vector2i: Vector2i,
    #[export]
    export_Rect2: Rect2,
    #[export]
    export_Rect2i: Rect2i,
    #[export]
    export_Vector3: Vector3,
    #[export]
    export_Vector3i: Vector3i,
    #[export]
    export_Transform2D: Transform2D,
    #[export]
    export_Vector4: Vector4,
    #[export]
    export_Vector4i: Vector4i,
    #[export]
    #[init(default = Plane::new(Vector3::new(1.0,0.0,0.0), 0.0))]
    export_Plane: Plane,
    #[export]
    export_Quaternion: Quaternion,
    #[export]
    export_Aabb: Aabb,
    #[export]
    export_Basis: Basis,
    #[export]
    export_Transform3D: Transform3D,
    #[export]
    export_Projection: Projection,
    #[export]
    export_Color: Color,
    #[export]
    export_StringName: StringName,
    #[export]
    export_NodePath: NodePath,
    // We do not allow exporting RIDs as they are useless when exported.
    // #[export]
    // export_Rid: Rid,
    #[export]
    export_Gd_Node: Option<Gd<Node>>,
    #[export]
    export_Gd_Resource: Option<Gd<Resource>>,
    // We do not allow exporting Callables as they are useless when exported
    // #[export]
    // export_Callable: Callable,
    #[export]
    export_Dictionary: Dictionary,
    #[export]
    export_VariantArray: VariantArray,
    #[export]
    export_PackedByteArray: PackedByteArray,
    #[export]
    export_PackedInt32Array: PackedInt32Array,
    #[export]
    export_PackedInt64Array: PackedInt64Array,
    #[export]
    export_PackedFloat32Array: PackedFloat32Array,
    #[export]
    export_PackedFloat64Array: PackedFloat64Array,
    #[export]
    export_PackedStringArray: PackedStringArray,
    #[export]
    export_PackedVector2Array: PackedVector2Array,
    #[export]
    export_PackedVector3Array: PackedVector3Array,
    #[export]
    export_PackedColorArray: PackedColorArray,

    // Exporting types nested in arrays
    #[export]
    export_array_bool: Array<bool>,
    #[export]
    export_array_i64: Array<i64>,
    #[export]
    export_array_i32: Array<i32>,
    #[export]
    export_array_i16: Array<i16>,
    #[export]
    export_array_i8: Array<i8>,
    #[export]
    export_array_u32: Array<u32>,
    #[export]
    export_array_u16: Array<u16>,
    #[export]
    export_array_u8: Array<u8>,
    #[export]
    export_array_f64: Array<f64>,
    #[export]
    export_array_f32: Array<f32>,
    #[export]
    export_array_GodotString: Array<GodotString>,
    #[export]
    export_array_Vector2: Array<Vector2>,
    #[export]
    export_array_Vector2i: Array<Vector2i>,
    #[export]
    export_array_Rect2: Array<Rect2>,
    #[export]
    export_array_Rect2i: Array<Rect2i>,
    #[export]
    export_array_Vector3: Array<Vector3>,
    #[export]
    export_array_Vector3i: Array<Vector3i>,
    #[export]
    export_array_Transform2D: Array<Transform2D>,
    #[export]
    export_array_Vector4: Array<Vector4>,
    #[export]
    export_array_Vector4i: Array<Vector4i>,
    #[export]
    export_array_Plane: Array<Plane>,
    #[export]
    export_array_Quaternion: Array<Quaternion>,
    #[export]
    export_array_Aabb: Array<Aabb>,
    #[export]
    export_array_Basis: Array<Basis>,
    #[export]
    export_array_Transform3D: Array<Transform3D>,
    #[export]
    export_array_Projection: Array<Projection>,
    #[export]
    export_array_Color: Array<Color>,
    #[export]
    export_array_StringName: Array<StringName>,
    #[export]
    export_array_NodePath: Array<NodePath>,
    #[export]
    export_array_Rid: Array<Rid>,
    #[export]
    export_array_Gd_Node: Array<Gd<Node>>,
    #[export]
    export_array_Gd_Resource: Array<Gd<Resource>>,
    #[export]
    export_array_Callable: Array<Callable>,
    #[export]
    export_array_Dictionary: Array<Dictionary>,
    #[export]
    export_array_VariantArray: Array<VariantArray>,
    #[export]
    export_array_PackedByteArray: Array<PackedByteArray>,
    #[export]
    export_array_PackedInt32Array: Array<PackedInt32Array>,
    #[export]
    export_array_PackedInt64Array: Array<PackedInt64Array>,
    #[export]
    export_array_PackedFloat32Array: Array<PackedFloat32Array>,
    #[export]
    export_array_PackedFloat64Array: Array<PackedFloat64Array>,
    #[export]
    export_array_PackedStringArray: Array<PackedStringArray>,
    #[export]
    export_array_PackedVector2Array: Array<PackedVector2Array>,
    #[export]
    export_array_PackedVector3Array: Array<PackedVector3Array>,
    #[export]
    export_array_PackedColorArray: Array<PackedColorArray>,

    // Exporting with custom hints
    #[export(file)]
    export_file: GodotString,
    #[export(file = "*.txt")]
    export_file_wildcard_txt: GodotString,
    #[export(global_file)]
    export_global_file: GodotString,
    #[export(global_file = "*.png")]
    export_global_file_wildcard_png: GodotString,
    #[export(dir)]
    export_dir: GodotString,
    #[export(global_dir)]
    export_global_dir: GodotString,
    #[export(multiline)]
    export_multiline: GodotString,
    #[export(range = (0.0, 20.0))]
    export_range_float_0_20: f64,
    // We're missing step currently.
    // #[export(range = (-10, 20 /* , 0.2 */))]
    // export_range_float_neg_10_20_02: f64,
    // we can only export ranges of floats currently
    // #[export(range = (0, 100, 1, "or_greater", "or_less"))]
    // export_range_int_0_100_1_or_greater_or_less: int,
    #[export(exp_easing)]
    export_exp_easing: f64,
    #[export(color_no_alpha)]
    export_color_no_alpha: Color,
    // Not implemented
    // #[export(node_path = ("Button", "TouchScreenButton"))]
    // export_node_path_button_touch_screen_button: NodePath,
    #[export(flags = (Fire, Water, Earth, Wind))]
    export_flags_fire_water_earth_wind: i64,
    #[export(flags = (Self = 4, Allies = 8, Foes = 16))]
    export_flags_self_4_allies_8_foes_16: i64,
    #[export(flags_2d_physics)]
    export_flags_2d_physics: i64,
    #[export(flags_2d_render)]
    export_flags_2d_render: i64,
    #[export(flags_2d_navigation)]
    export_flags_2d_navigation: i64,
    #[export(flags_3d_physics)]
    export_flags_3d_physics: i64,
    #[export(flags_3d_render)]
    export_flags_3d_render: i64,
    #[export(flags_3d_navigation)]
    export_flags_3d_navigation: i64,
    #[export(enum = (Warrior, Magician, Thief))]
    export_enum_int_warrior_magician_thief: i64,
    #[export(enum = (Slow = 30, Average = 60, VeryFast = 200))]
    export_enum_int_slow_30_average_60_very_fast_200: i64,
    #[export(enum = (Rebecca, Mary, Leah))]
    export_enum_string_rebecca_mary_leah: GodotString,
}

#[godot_api]
impl PropertyTemplateRust {}

#[itest]
fn property_template_test(ctx: &TestContext) {
    let rust_properties = Gd::<PropertyTemplateRust>::new_default();
    let gdscript_properties = ctx.property_template.clone();

    // Accumulate errors so we can catch all of them in one go.
    let mut errors: Vec<String> = Vec::new();
    let mut properties: HashMap<String, Dictionary> = HashMap::new();

    for property in rust_properties.get_property_list().iter_shared() {
        let name = property
            .get("name")
            .unwrap()
            .to::<GodotString>()
            .to_string();
        if name.starts_with("property_") || name.starts_with("export_") {
            properties.insert(name, property);
        }
    }

    assert!(!properties.is_empty());

    for property in gdscript_properties.get_property_list().iter_shared() {
        let name = property
            .get("name")
            .unwrap()
            .to::<GodotString>()
            .to_string();

        let Some(mut rust_prop) = properties.remove(&name) else {
            continue;
        };

        let mut rust_usage = rust_prop.get("usage").unwrap().to::<i64>();

        // the GDSscript variables are script variables, and so have `PROPERTY_USAGE_SCRIPT_VARIABLE` set.
        if rust_usage == PropertyUsageFlags::PROPERTY_USAGE_STORAGE.ord() as i64 {
            // `PROPERTY_USAGE_SCRIPT_VARIABLE` does the same thing as `PROPERTY_USAGE_STORAGE` and so
            // GDScript doesn't set both if it doesn't need to.
            rust_usage = PropertyUsageFlags::PROPERTY_USAGE_SCRIPT_VARIABLE.ord() as i64
        } else {
            rust_usage |= PropertyUsageFlags::PROPERTY_USAGE_SCRIPT_VARIABLE.ord() as i64;
        }

        rust_prop.set("usage", rust_usage);

        if rust_prop != property {
            errors.push(format!(
                "mismatch in property {name}, gdscript: {property:?}, rust: {rust_prop:?}"
            ));
        }
    }

    assert!(
        properties.is_empty(),
        "not all properties were matched, missing: {properties:?}"
    );

    assert!(errors.is_empty(), "{}", errors.join("\n"));

    rust_properties.free();
}
