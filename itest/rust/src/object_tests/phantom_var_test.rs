/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use godot::builtin::Callable;
use godot::classes::RefCounted;
use godot::obj::{Base, NewGd, WithBaseField};
use godot::register::property::{ExportToolButton, PhantomVar};
use godot::register::{godot_api, GodotClass};

use crate::framework::itest;

#[derive(GodotClass)]
#[class(init)]
struct HasPhantomVar {
    #[var(get = get_read_only, no_set)]
    read_only: PhantomVar<i64>,

    #[var(get, set)]
    read_write: PhantomVar<i64>,

    #[var(get = get_engine_enum, set = set_engine_enum)]
    read_write_engine_enum: PhantomVar<godot::global::VerticalAlignment>,

    #[var(get = get_bit_enum, set = set_bit_enum)]
    read_write_bit_enum: PhantomVar<godot::global::KeyModifierMask>,

    value: i64,

    #[init(val = godot::global::VerticalAlignment::CENTER)]
    engine_enum_value: godot::global::VerticalAlignment,

    #[init(val = godot::global::KeyModifierMask::ALT|godot::global::KeyModifierMask::CTRL)]
    bit_enum_value: godot::global::KeyModifierMask,
}

#[godot_api]
impl HasPhantomVar {
    #[func]
    fn get_read_only(&self) -> i64 {
        self.value
    }

    #[func]
    fn get_read_write(&self) -> i64 {
        self.value
    }

    #[func]
    fn set_read_write(&mut self, value: i64) {
        self.value = value;
    }

    #[func]
    fn get_engine_enum(&self) -> godot::global::VerticalAlignment {
        self.engine_enum_value
    }

    #[func]
    fn set_engine_enum(&mut self, value: godot::global::VerticalAlignment) {
        self.engine_enum_value = value;
    }

    #[func]
    fn get_bit_enum(&self) -> godot::global::KeyModifierMask {
        self.bit_enum_value
    }

    #[func]
    fn set_bit_enum(&mut self, value: godot::global::KeyModifierMask) {
        self.bit_enum_value = value;
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(since_api = "4.4")]
mod export_tool_button_test {
    use super::*;
    #[derive(GodotClass)]
    #[class(init, tool)]
    struct ToolButtonExporter {
        #[export_tool_button(fn = |this: &mut Self| this.val = 42)]
        my_tool_button: ExportToolButton,

        #[export_tool_button(fn = Self::my_fn)]
        other_tool_button: ExportToolButton,

        val: i32,
        base: Base<RefCounted>,
    }

    impl ToolButtonExporter {
        fn my_fn(&mut self) {
            self.val = 33;
        }
    }

    #[itest]
    fn test_tool_button() {
        let tool_button_exporter = ToolButtonExporter::new_gd();
        let tool_button_callable = tool_button_exporter.get("my_tool_button").to::<Callable>();
        tool_button_callable.call(&[]);
        assert_eq!(tool_button_exporter.bind().val, 42);

        let tool_button_callable = tool_button_exporter
            .get("other_tool_button")
            .to::<Callable>();
        tool_button_callable.call(&[]);
        assert_eq!(tool_button_exporter.bind().val, 33);
    }
}
