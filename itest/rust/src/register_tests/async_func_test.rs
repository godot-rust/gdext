/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::builtin::{Color, StringName, Vector2, Vector3};
use godot::classes::ClassDb;
use godot::prelude::*;
use godot::task::spawn_with_result;

use std::time::Duration;
use tokio::time;

// Test tokio runtime integration

// Basic async function tests
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct AsyncTestClass;

#[godot_api]
impl AsyncTestClass {
    #[async_func]
    async fn async_vector2_multiply(input: Vector2) -> Vector2 {
        // Use real tokio sleep to test tokio runtime integration
        time::sleep(Duration::from_millis(10)).await;
        Vector2::new(input.x * 2.0, input.y * 2.0)
    }

    #[async_func]
    async fn async_vector3_normalize(input: Vector3) -> Vector3 {
        // Use real tokio sleep to test tokio runtime integration
        time::sleep(Duration::from_millis(5)).await;
        input.normalized()
    }

    #[async_func]
    async fn async_color_brighten(color: Color, amount: f32) -> Color {
        // Use real tokio sleep to test tokio runtime integration
        time::sleep(Duration::from_millis(8)).await;
        Color::from_rgb(
            (color.r + amount).min(1.0),
            (color.g + amount).min(1.0),
            (color.b + amount).min(1.0),
        )
    }

    #[async_func]
    async fn async_compute_sum(a: i32, b: i32) -> i32 {
        // Use real tokio sleep to test tokio runtime integration
        time::sleep(Duration::from_millis(12)).await;
        a + b
    }

    #[async_func]
    async fn async_get_magic_number() -> i32 {
        time::sleep(Duration::from_millis(15)).await;
        42
    }
}

// Simple async runtime test
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct AsyncRuntimeTestClass;

#[godot_api]
impl AsyncRuntimeTestClass {
    #[async_func]
    async fn test_simple_async_chain() -> StringName {
        // Test chaining real tokio async operations
        time::sleep(Duration::from_millis(20)).await;
        time::sleep(Duration::from_millis(30)).await;

        StringName::from("Simple async chain test passed")
    }

    #[async_func]
    async fn test_simple_async() -> i32 {
        // Test real tokio async computation
        time::sleep(Duration::from_millis(25)).await;
        let result1 = 42;
        time::sleep(Duration::from_millis(35)).await;
        let result2 = 58;
        result1 + result2
    }
}

#[itest]
fn async_func_registration() {
    let class_name = StringName::from("AsyncTestClass");
    assert!(ClassDb::singleton().class_exists(&class_name));

    // Check that async methods are registered
    let methods = ClassDb::singleton().class_get_method_list(&class_name);
    let method_names: Vec<String> = methods
        .iter_shared()
        .map(|method_dict| {
            // Extract method name from dictionary
            let name_variant = method_dict.get("name").unwrap_or_default();
            name_variant.to_string()
        })
        .collect();

    // Verify our async methods are registered
    assert!(method_names
        .iter()
        .any(|name| name.contains("async_vector2_multiply")));
    assert!(method_names
        .iter()
        .any(|name| name.contains("async_vector3_normalize")));
    assert!(method_names
        .iter()
        .any(|name| name.contains("async_color_brighten")));
    assert!(method_names
        .iter()
        .any(|name| name.contains("async_compute_sum")));
}

#[itest]
fn async_func_signature_validation() {
    let class_name = StringName::from("AsyncTestClass");

    // Verify that async methods are registered with correct names
    assert!(ClassDb::singleton()
        .class_has_method(&class_name, &StringName::from("async_vector2_multiply")));
    assert!(ClassDb::singleton()
        .class_has_method(&class_name, &StringName::from("async_vector3_normalize")));
    assert!(ClassDb::singleton()
        .class_has_method(&class_name, &StringName::from("async_color_brighten")));
    assert!(
        ClassDb::singleton().class_has_method(&class_name, &StringName::from("async_compute_sum"))
    );
    assert!(ClassDb::singleton()
        .class_has_method(&class_name, &StringName::from("async_get_magic_number")));
}

#[itest]
fn async_runtime_class_registration() {
    let class_name = StringName::from("AsyncRuntimeTestClass");
    assert!(ClassDb::singleton().class_exists(&class_name));

    // Verify that async runtime test methods are registered
    assert!(ClassDb::singleton()
        .class_has_method(&class_name, &StringName::from("test_simple_async_chain")));
    assert!(
        ClassDb::singleton().class_has_method(&class_name, &StringName::from("test_simple_async"))
    );
}

#[itest]
fn test_spawn_with_result_signal_emission() {
    // Test that spawn_with_result creates an object with a "finished" signal
    let signal_emitter = spawn_with_result(async {
        time::sleep(Duration::from_millis(5)).await;
        42i32
    });

    // Check that the object exists
    println!(
        "Signal emitter instance ID: {:?}",
        signal_emitter.instance_id()
    );

    // TODO: We should verify signal emission, but that's complex in a direct test
    // The GDScript tests will verify the full functionality
    println!("Signal emitter created successfully: {signal_emitter:?}");
}

// Test real tokio ecosystem integration
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct AsyncNetworkTestClass;

#[godot_api]
impl AsyncNetworkTestClass {
    #[async_func]
    async fn async_http_request() -> i32 {
        // Test real tokio ecosystem with HTTP request
        match reqwest::get("https://httpbin.org/json").await {
            Ok(response) => response.status().as_u16() as i32,
            Err(_e) => -1,
        }
    }

    #[async_func]
    async fn async_concurrent_requests() -> i32 {
        // Test concurrent tokio operations
        let (res1, res2) = tokio::join!(
            reqwest::get("https://httpbin.org/delay/1"),
            reqwest::get("https://httpbin.org/delay/1")
        );

        match (res1, res2) {
            (Ok(r1), Ok(r2)) => (r1.status().as_u16() + r2.status().as_u16()) as i32,
            _ => -1,
        }
    }
}
