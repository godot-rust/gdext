/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::IObject;
use godot::obj::{Base, Gd, NewAlloc};
use godot::register::{godot_api, GodotClass};

use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Object)]
struct MultipleImplBlocks {}

#[godot_api]
impl IObject for MultipleImplBlocks {
    fn init(_base: Base<Self::Base>) -> Self {
        Self {}
    }
}

#[godot_api]
impl MultipleImplBlocks {
    #[func]
    fn first(&self) -> String {
        "1st result".to_string()
    }
}

#[godot_api(secondary)]
impl MultipleImplBlocks {
    #[func]
    fn second(&self) -> String {
        "2nd result".to_string()
    }
}

#[godot_api(secondary)]
impl MultipleImplBlocks {
    #[func]
    fn third(&self) -> String {
        "3rd result".to_string()
    }
}

/// Test that multiple inherent '#[godot_api]' impl blocks can be registered.
/// https://github.com/godot-rust/gdext/pull/927
#[itest]
fn godot_api_multiple_impl_blocks() {
    let mut obj: Gd<MultipleImplBlocks> = MultipleImplBlocks::new_alloc();

    fn call_and_check_result(
        gd: &mut Gd<MultipleImplBlocks>,
        method_name: &str,
        expected_result: &str,
    ) {
        assert!(gd.has_method(method_name));
        let result = gd.call(method_name, &[]);
        let result_as_string = result.try_to::<String>();
        assert!(result_as_string.is_ok());
        assert_eq!(result_as_string.unwrap(), expected_result);
    }

    // Just call all three methods; if that works, then they have all been correctly registered.
    call_and_check_result(&mut obj, "first", "1st result");
    call_and_check_result(&mut obj, "second", "2nd result");
    call_and_check_result(&mut obj, "third", "3rd result");

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
