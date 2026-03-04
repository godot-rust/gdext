/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

struct HotReload;

#[gdextension]
unsafe impl ExtensionLibrary for HotReload {
    fn on_stage_init(stage: InitStage) {
        println!("[Rust]      Init stage {stage:?}");
    }

    fn on_stage_deinit(stage: InitStage) {
        println!("[Rust]      Deinit stage {stage:?}");
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Node)]
struct Reloadable {
    #[export]
    #[init(val = Planet::Earth)]
    favorite_planet: Planet,

    #[init(val = NoDefault::obtain())]
    _other_object: Gd<NoDefault>,
}

#[godot_api]
impl Reloadable {
    #[func]
    #[rustfmt::skip]
    // DO NOT MODIFY FOLLOWING LINE -- replaced by hot-reload test. Hence #[rustfmt::skip] above.
    fn get_number(&self) -> i64 { 100 }

    #[func]
    fn from_string(s: GString) -> Gd<Self> {
        Gd::from_object(Reloadable {
            favorite_planet: Planet::from_godot(s),
            _other_object: NoDefault::obtain(),
        })
    }
}

// no_init reloadability - https://github.com/godot-rust/gdext/issues/874.
#[derive(GodotClass)]
#[class(no_init, base=Node)]
struct NoDefault {}

#[godot_api]
impl NoDefault {
    #[func]
    fn obtain() -> Gd<Self> {
        Gd::from_object(NoDefault {})
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, GodotConvert, Var, Export)]
#[godot(via = GString)]
enum Planet {
    Earth,
    Mars,
    Venus,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Signal soundness: https://github.com/godot-rust/gdext/pull/1512.

#[cfg(feature = "signal-test")]
pub use signal_test::*;

#[cfg(feature = "signal-test")]
mod signal_test {
    use godot::classes::notify::ObjectNotification;

    use super::*;

    #[derive(GodotClass)]
    #[class(init, tool, base = Object)]
    struct Signaller {
        #[var]
        value: i64,
        base: Base<Object>,
    }

    #[godot_api]
    impl IObject for Signaller {
        fn on_notification(&mut self, what: ObjectNotification) {
            // Recreate signal after hot reload.
            // Doesn't really matter much for this test - it does NOT replace previous signal (which should be disconnected at this point).
            if what == ObjectNotification::EXTENSION_RELOADED {
                self.signals()
                    .reloadable_signal()
                    .connect_self(|this, val| this.value = val);
            }
        }
    }

    #[godot_api]
    impl Signaller {
        #[func]
        fn initialize_connections(&mut self) {
            self.signals()
                .reloadable_signal()
                .connect_self(|this, val| this.value = val);

            // Given RefCounted will be instantly freed after this function execution,
            // but signal connection will be registered and pruned upon hot reload.
            let some_refcounted = RefCounted::new_gd();
            some_refcounted
                .signals()
                .property_list_changed()
                .connect(|| godot_print!("Henlo!"));
        }

        #[signal]
        fn reloadable_signal(val: i64);
    }
}
