/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())]
#![allow(clippy::non_minimal_cfg)]

use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
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

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct GdSelfReference {
    internal_value: i32,

    #[base]
    base: Base<RefCounted>,
}

#[godot_api]
impl GdSelfReference {
    // A signal that will be looped back to update_internal through gdscript.
    #[signal]
    fn update_internal_signal(new_internal: i32);

    #[func]
    fn update_internal(&mut self, new_value: i32) {
        self.internal_value = new_value;
    }

    #[func]
    #[rustfmt::skip]
    fn func_shouldnt_panic_with_segmented_path_attribute() -> bool {
        true
    }

    #[cfg(all())]
    #[func]
    fn func_recognized_with_simple_path_attribute_above_func_attr() -> bool {
        true
    }

    #[func]
    #[cfg(all())]
    fn func_recognized_with_simple_path_attribute_below_func_attr() -> bool {
        true
    }

    #[func]
    fn funcs_above_are_kept() -> bool {
        let f2 = Self::func_recognized_with_simple_path_attribute_above_func_attr();
        let f1 = Self::func_recognized_with_simple_path_attribute_below_func_attr();

        f1 && f2
    }

    #[signal]
    #[rustfmt::skip]
    fn signal_shouldnt_panic_with_segmented_path_attribute();

    #[cfg(all())]
    #[signal]
    fn signal_recognized_with_simple_path_attribute_above_signal_attr();

    #[signal]
    #[cfg(all())]
    fn signal_recognized_with_simple_path_attribute_below_signal_attr();

    #[func]
    fn fail_to_update_internal_value_due_to_conflicting_borrow(
        &mut self,
        new_internal: i32,
    ) -> i32 {
        // Since a self reference is held while the signal is emitted, when
        // GDScript tries to call update_internal(), there will be a failure due
        // to the double borrow and self.internal_value won't be changed.
        self.base.emit_signal(
            "update_internal_signal".into(),
            &[new_internal.to_variant()],
        );
        self.internal_value
    }

    #[func(gd_self)]
    fn succeed_at_updating_internal_value(mut this: Gd<Self>, new_internal: i32) -> i32 {
        // Since this isn't bound while the signal is emitted, GDScript will succeed at calling
        // update_internal() and self.internal_value will be changed.
        this.emit_signal(
            "update_internal_signal".into(),
            &[new_internal.to_variant()],
        );
        return this.bind().internal_value;
    }

    #[func(gd_self)]
    fn takes_gd_as_equivalent(mut this: Gd<GdSelfReference>) -> bool {
        this.bind_mut();
        true
    }

    #[func(gd_self)]
    fn takes_gd_as_self_no_return_type(this: Gd<GdSelfReference>) {
        this.bind();
    }
}

#[godot_api]
impl RefCountedVirtual for GdSelfReference {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            internal_value: 0,
            base,
        }
    }
}
