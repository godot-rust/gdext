/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::builtin::{StringName, Vector2};

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
    async fn async_compute_sum(a: i32, b: i32) -> i32 {
        // Use real tokio sleep to test tokio runtime integration
        time::sleep(Duration::from_millis(12)).await;
        a + b
    }

    #[async_func]
    async fn async_get_magic_number() -> i32 {
        // Test with a short tokio sleep
        time::sleep(Duration::from_millis(15)).await;
        42
    }

    #[async_func]
    async fn async_get_message() -> StringName {
        // Test async with string return
        time::sleep(Duration::from_millis(20)).await;
        StringName::from("async message")
    }
}

// Note: AsyncRuntimeTestClass was removed as it was redundant with AsyncTestClass

#[itest]
fn test_spawn_with_result_signal_emission() {
    // Test that spawn_with_result creates an object with a "finished" signal
    let signal_emitter = spawn_with_result(async {
        time::sleep(Duration::from_millis(5)).await;
        42i32
    });

    // Verify that the object exists
    assert!(signal_emitter.is_instance_valid());

    // TODO: We should verify signal emission, but that's complex in a direct test
    // The GDScript tests will verify the full functionality
}

// Simple test for async instance methods
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct SimpleAsyncClass {
    base: Base<RefCounted>,
    value: i32,
}

#[godot_api]
impl SimpleAsyncClass {
    #[func]
    fn set_value(&mut self, new_value: i32) {
        self.value = new_value;
    }

    #[func]
    fn get_value(&self) -> i32 {
        self.value
    }

    // Test single async instance method
    #[async_func]
    async fn async_get_value(&self) -> i32 {
        time::sleep(Duration::from_millis(10)).await;
        self.value
    }
}
