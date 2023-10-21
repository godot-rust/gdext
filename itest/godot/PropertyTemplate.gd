# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

class_name PropertyTemplateGDScript
extends Node

# Base types

var property_bool: bool
var property_i64: int
var property_i32: int
var property_i16: int
var property_i8: int
var property_u32: int
var property_u16: int
var property_u8: int
var property_f64: float
var property_f32: float
var property_GodotString: String
var property_Vector2: Vector2
var property_Vector2i: Vector2i
var property_Rect2: Rect2
var property_Rect2i: Rect2i
var property_Vector3: Vector3
var property_Vector3i: Vector3i
var property_Transform2D: Transform2D
var property_Vector4: Vector4
var property_Vector4i: Vector4i
var property_Plane: Plane
var property_Quaternion: Quaternion
var property_Aabb: AABB
var property_Basis: Basis
var property_Transform3D: Transform3D
var property_Projection: Projection
var property_Color: Color
var property_StringName: StringName
var property_NodePath: NodePath
var property_Rid: RID
var property_Gd_Node: Node
var property_Gd_Resource: Resource
var property_Callable: Callable
var property_Dictionary: Dictionary
var property_VariantArray: Array
var property_PackedByteArray: PackedByteArray
var property_PackedInt32Array: PackedInt32Array
var property_PackedInt64Array: PackedInt64Array
var property_PackedFloat32Array: PackedFloat32Array
var property_PackedFloat64Array: PackedFloat64Array
var property_PackedStringArray: PackedStringArray
var property_PackedVector2Array: PackedVector2Array
var property_PackedVector3Array: PackedVector3Array
var property_PackedColorArray: PackedColorArray

# Types nested in arrays

var property_array_bool: Array[bool]
var property_array_i64: Array[int]
var property_array_i32: Array[int]
var property_array_i16: Array[int]
var property_array_i8: Array[int]
var property_array_u32: Array[int]
var property_array_u16: Array[int]
var property_array_u8: Array[int]
var property_array_f64: Array[float]
var property_array_f32: Array[float]
var property_array_GodotString: Array[String]
var property_array_Vector2: Array[Vector2]
var property_array_Vector2i: Array[Vector2i]
var property_array_Rect2: Array[Rect2]
var property_array_Rect2i: Array[Rect2i]
var property_array_Vector3: Array[Vector3]
var property_array_Vector3i: Array[Vector3i]
var property_array_Transform2D: Array[Transform2D]
var property_array_Vector4: Array[Vector4]
var property_array_Vector4i: Array[Vector4i]
var property_array_Plane: Array[Plane]
var property_array_Quaternion: Array[Quaternion]
var property_array_Aabb: Array[AABB]
var property_array_Basis: Array[Basis]
var property_array_Transform3D: Array[Transform3D]
var property_array_Projection: Array[Projection]
var property_array_Color: Array[Color]
var property_array_StringName: Array[StringName]
var property_array_NodePath: Array[NodePath]
var property_array_Rid: Array[RID]
var property_array_Gd_Node: Array[Node]
var property_array_Gd_Resource: Array[Resource]
var property_array_Callable: Array[Callable]
var property_array_Dictionary: Array[Dictionary]
var property_array_VariantArray: Array[Array]
var property_array_PackedByteArray: Array[PackedByteArray]
var property_array_PackedInt32Array: Array[PackedInt32Array]
var property_array_PackedInt64Array: Array[PackedInt64Array]
var property_array_PackedFloat32Array: Array[PackedFloat32Array]
var property_array_PackedFloat64Array: Array[PackedFloat64Array]
var property_array_PackedStringArray: Array[PackedStringArray]
var property_array_PackedVector2Array: Array[PackedVector2Array]
var property_array_PackedVector3Array: Array[PackedVector3Array]
var property_array_PackedColorArray: Array[PackedColorArray]

# Exporting base types

@export var export_bool: bool
@export var export_i64: int
@export var export_i32: int
@export var export_i16: int
@export var export_i8: int
@export var export_u32: int
@export var export_u16: int
@export var export_u8: int
@export var export_f64: float
@export var export_f32: float
@export var export_GodotString: String
@export var export_Vector2: Vector2
@export var export_Vector2i: Vector2i
@export var export_Rect2: Rect2
@export var export_Rect2i: Rect2i
@export var export_Vector3: Vector3
@export var export_Vector3i: Vector3i
@export var export_Transform2D: Transform2D
@export var export_Vector4: Vector4
@export var export_Vector4i: Vector4i
@export var export_Plane: Plane
@export var export_Quaternion: Quaternion
@export var export_Aabb: AABB
@export var export_Basis: Basis
@export var export_Transform3D: Transform3D
@export var export_Projection: Projection
@export var export_Color: Color
@export var export_StringName: StringName
@export var export_NodePath: NodePath
@export var export_Rid: RID
@export var export_Gd_Node: Node
@export var export_Gd_Resource: Resource
@export var export_Callable: Callable
@export var export_Dictionary: Dictionary
@export var export_VariantArray: Array
@export var export_PackedByteArray: PackedByteArray
@export var export_PackedInt32Array: PackedInt32Array
@export var export_PackedInt64Array: PackedInt64Array
@export var export_PackedFloat32Array: PackedFloat32Array
@export var export_PackedFloat64Array: PackedFloat64Array
@export var export_PackedStringArray: PackedStringArray
@export var export_PackedVector2Array: PackedVector2Array
@export var export_PackedVector3Array: PackedVector3Array
@export var export_PackedColorArray: PackedColorArray

# Exporting types nested in arrays

@export var export_array_bool: Array[bool]
@export var export_array_i64: Array[int]
@export var export_array_i32: Array[int]
@export var export_array_i16: Array[int]
@export var export_array_i8: Array[int]
@export var export_array_u32: Array[int]
@export var export_array_u16: Array[int]
@export var export_array_u8: Array[int]
@export var export_array_f64: Array[float]
@export var export_array_f32: Array[float]
@export var export_array_GodotString: Array[String]
@export var export_array_Vector2: Array[Vector2]
@export var export_array_Vector2i: Array[Vector2i]
@export var export_array_Rect2: Array[Rect2]
@export var export_array_Rect2i: Array[Rect2i]
@export var export_array_Vector3: Array[Vector3]
@export var export_array_Vector3i: Array[Vector3i]
@export var export_array_Transform2D: Array[Transform2D]
@export var export_array_Vector4: Array[Vector4]
@export var export_array_Vector4i: Array[Vector4i]
@export var export_array_Plane: Array[Plane]
@export var export_array_Quaternion: Array[Quaternion]
@export var export_array_Aabb: Array[AABB]
@export var export_array_Basis: Array[Basis]
@export var export_array_Transform3D: Array[Transform3D]
@export var export_array_Projection: Array[Projection]
@export var export_array_Color: Array[Color]
@export var export_array_StringName: Array[StringName]
@export var export_array_NodePath: Array[NodePath]
@export var export_array_Rid: Array[RID]
@export var export_array_Gd_Node: Array[Node]
@export var export_array_Gd_Resource: Array[Resource]
@export var export_array_Callable: Array[Callable]
@export var export_array_Dictionary: Array[Dictionary]
@export var export_array_VariantArray: Array[Array]
@export var export_array_PackedByteArray: Array[PackedByteArray]
@export var export_array_PackedInt32Array: Array[PackedInt32Array]
@export var export_array_PackedInt64Array: Array[PackedInt64Array]
@export var export_array_PackedFloat32Array: Array[PackedFloat32Array]
@export var export_array_PackedFloat64Array: Array[PackedFloat64Array]
@export var export_array_PackedStringArray: Array[PackedStringArray]
@export var export_array_PackedVector2Array: Array[PackedVector2Array]
@export var export_array_PackedVector3Array: Array[PackedVector3Array]
@export var export_array_PackedColorArray: Array[PackedColorArray]

# Exporting with custom hints

@export_file var export_file: String
@export_file("*.txt") var export_file_wildcard_txt: String
@export_global_file var export_global_file: String
@export_global_file("*.png") var export_global_file_wildcard_png: String
@export_dir var export_dir: String
@export_global_dir var export_global_dir: String
@export_multiline var export_multiline: String
@export_range(0, 20) var export_range_float_0_20: float
@export_range(-10, 20, 0.2) var export_range_float_neg_10_20_02: float
@export_range(0, 100, 1, "or_greater", "or_less") var export_range_int_0_100_1_or_greater_or_less: int
@export_exp_easing var export_exp_easing: float
@export_color_no_alpha var export_color_no_alpha: Color
@export_node_path("Button", "TouchScreenButton") var export_node_path_button_touch_screen_button: NodePath
@export_flags("Fire", "Water", "Earth", "Wind") var export_flags_fire_water_earth_wind: int
@export_flags("Self:4", "Allies:8", "Foes:16") var export_flags_self_4_allies_8_foes_16: int
@export_flags_2d_physics var export_flags_2d_physics: int
@export_flags_2d_render var export_flags_2d_render: int
@export_flags_2d_navigation var export_flags_2d_navigation: int
@export_flags_3d_physics var export_flags_3d_physics: int
@export_flags_3d_render var export_flags_3d_render: int
@export_flags_3d_navigation var export_flags_3d_navigation: int
@export_enum("Warrior", "Magician", "Thief") var export_enum_int_warrior_magician_thief: int
@export_enum("Slow:30", "Average:60", "VeryFast:200") var export_enum_int_slow_30_average_60_very_fast_200: int
@export_enum("Rebecca", "Mary", "Leah") var export_enum_string_rebecca_mary_leah: String
