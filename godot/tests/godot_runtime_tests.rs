//! Tests for godot-rust core functionality using the embedded runtime.
//!
//! These tests run within a real Godot engine instance, allowing us to test
//! the actual FFI bindings and Godot API interactions.
//!
//! Run with: `cargo test --features api-4-3 --test godot_runtime_tests`
//! The api-4-3 feature is required to match the embedded Godot runtime version.

use godot::prelude::*;
use godot_testability_runtime::godot_test_main;
use godot_testability_runtime::prelude::*;

fn test_gd_creation(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test that we can create Gd instances
    let mut node = Node::new_alloc();
    assert!(node.is_instance_valid());

    node.set_name("TestNode");
    assert_eq!(node.get_name().to_string(), "TestNode");

    node.queue_free();
    Ok(())
}

fn test_variant_conversions(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test Variant conversions
    let int_variant = Variant::from(42i64);
    assert_eq!(int_variant.get_type(), VariantType::INT);
    assert_eq!(int_variant.to::<i64>(), 42);

    let string_variant = Variant::from(GString::from("test"));
    assert_eq!(string_variant.get_type(), VariantType::STRING);
    assert_eq!(string_variant.to::<GString>(), GString::from("test"));

    Ok(())
}

fn test_builtin_types(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test Vector types
    let v2 = Vector2::new(1.0, 2.0);
    let v3 = Vector3::new(1.0, 2.0, 3.0);
    let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);

    assert_eq!(v2.x, 1.0);
    assert_eq!(v3.y, 2.0);
    assert_eq!(v4.w, 4.0);

    // Test Color
    let color = Color::from_rgb(1.0, 0.5, 0.0);
    assert!((color.r - 1.0).abs() < f32::EPSILON);
    assert!((color.g - 0.5).abs() < f32::EPSILON);

    // Test Transform types
    let transform2d = Transform2D::IDENTITY;
    assert_eq!(transform2d, Transform2D::IDENTITY);

    let transform3d = Transform3D::IDENTITY;
    assert_eq!(transform3d, Transform3D::IDENTITY);

    Ok(())
}

fn test_string_conversions(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test GString conversions
    let rust_string = String::from("Hello, Godot!");
    let gstring = GString::from(&rust_string);
    assert_eq!(gstring.to_string(), rust_string);

    // Test StringName
    let string_name = StringName::from("test_name");
    assert_eq!(string_name.to_string(), "test_name");

    // Test NodePath
    let node_path = NodePath::from("Parent/Child/Grandchild");
    assert!(!node_path.is_empty());

    Ok(())
}

fn test_packed_arrays(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test PackedByteArray
    let mut byte_array = PackedByteArray::new();
    byte_array.push(1);
    byte_array.push(2);
    byte_array.push(3);
    assert_eq!(byte_array.len(), 3);
    assert_eq!(byte_array.get(0), Some(1));

    // Test PackedInt32Array
    let mut int_array = PackedInt32Array::new();
    int_array.push(100);
    int_array.push(200);
    assert_eq!(int_array.len(), 2);
    assert_eq!(int_array.get(1), Some(200));

    // Test PackedFloat32Array
    let mut float_array = PackedFloat32Array::new();
    float_array.push(1.5);
    float_array.push(2.5);
    assert_eq!(float_array.len(), 2);
    assert_eq!(float_array.get(0), Some(1.5));

    // Test PackedStringArray
    let mut string_array = PackedStringArray::new();
    string_array.push(&GString::from("first"));
    string_array.push(&GString::from("second"));
    assert_eq!(string_array.len(), 2);
    assert_eq!(string_array.get(0), Some(GString::from("first")));

    // Test PackedVector2Array
    let mut vec2_array = PackedVector2Array::new();
    vec2_array.push(Vector2::new(1.0, 2.0));
    vec2_array.push(Vector2::new(3.0, 4.0));
    assert_eq!(vec2_array.len(), 2);

    Ok(())
}

fn test_dictionary_and_array(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test Dictionary
    let mut dict = Dictionary::new();
    dict.set("key", 42);
    dict.set("name", GString::from("test"));

    assert_eq!(dict.len(), 2);
    assert_eq!(dict.get("key"), Some(Variant::from(42)));
    assert_eq!(dict.get("name"), Some(Variant::from(GString::from("test"))));

    // Test Array<Variant>
    let mut array: Array<Variant> = Array::new();
    array.push(&Variant::from(1));
    array.push(&Variant::from(GString::from("two")));
    array.push(&Variant::from(3.0));

    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0), Some(Variant::from(1)));

    Ok(())
}

fn test_callable_creation(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    // Test Callable creation
    let mut node = Node::new_alloc();
    let callable = Callable::from_object_method(&node, "set_name");

    assert!(callable.is_valid());

    node.queue_free();
    Ok(())
}

fn test_signal_operations(_scene_tree: &mut Gd<SceneTree>) -> TestResult<()> {
    use std::sync::{Arc, Mutex};

    let mut node = Node::new_alloc();
    let signal_received = Arc::new(Mutex::new(false));

    // Connect to the ready signal
    let signal_received_clone = signal_received.clone();
    node.connect(
        "ready",
        &Callable::from_local_fn("test_ready", move |_args| {
            *signal_received_clone.lock().unwrap() = true;
            Ok(Variant::nil())
        }),
    );

    // Emit the signal
    node.emit_signal("ready", &[]);

    // Check that the signal was received
    assert!(*signal_received.lock().unwrap());

    node.queue_free();
    Ok(())
}

godot_test_main! {
    test_gd_creation,
    test_variant_conversions,
    test_builtin_types,
    test_string_conversions,
    test_packed_arrays,
    test_dictionary_and_array,
    test_callable_creation,
    test_signal_operations,
}