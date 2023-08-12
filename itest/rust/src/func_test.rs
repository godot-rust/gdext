/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct FuncRename;

#[godot_api]
impl FuncRename {
    #[func(rename=is_true)]
    fn long_function_name_for_is_true(&self) -> bool {
        true
    }

    #[func(rename=give_one)]
    fn give_one_inner(&self) -> i32 {
        self.give_one()
    }

    #[func(rename=spell_static)]
    fn renamed_static() -> GodotString {
        GodotString::from("static")
    }
}

impl FuncRename {
    /// Unused but present to demonstrate how `rename = ...` can be used to avoid name clashes.
    #[allow(dead_code)]
    fn is_true(&self) -> bool {
        false
    }

    fn give_one(&self) -> i32 {
        1
    }
}

#[godot_api]
impl RefCountedVirtual for FuncRename {
    fn init(_base: Base<Self::Base>) -> Self {
        Self
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct DefaultParamsPrimitives;

#[godot_api]
impl DefaultParamsPrimitives {
    #[func(defaults = [1, 2])]
    fn add_ints(&self, a: Variant, b: Variant) -> i32 {
        println!("{:?}", a.get_type());
        println!("{:?}", b.get_type());
        (if a.is_nil() { 1 } else { a.to::<i32>() }) + (if b.is_nil() { 2 } else { b.to::<i32>() })
    }

    #[func(defaults = ["hello"])]
    fn pass_string(&self, text: GodotString) -> GodotString {
        text
    }
}

#[godot_api]
impl RefCountedVirtual for DefaultParamsPrimitives {
    fn init(_base: Base<Self::Base>) -> Self {
        Self
    }
}
