/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::{expect_panic, itest};
use godot::bind::{godot_api, GodotClass};
use godot::engine::notify::NodeNotification;
use godot::engine::INode;

use godot::obj::{Gd, OnReady};

#[itest]
fn onready_deref() {
    let mut l = OnReady::<i32>::new(|| 42);
    godot::private::auto_init(&mut l);

    // DerefMut
    let mut_ref: &mut i32 = &mut l;
    assert_eq!(*mut_ref, 42);

    // Deref
    let l = l;
    let shared_ref: &i32 = &l;
    assert_eq!(*shared_ref, 42);
}

#[itest]
fn onready_deref_on_uninit() {
    expect_panic("Deref on uninit fails", || {
        let l = OnReady::<i32>::new(|| 42);
        let _ref: &i32 = &l;
    });

    expect_panic("DerefMut on uninit fails", || {
        let mut l = OnReady::<i32>::new(|| 42);
        let _ref: &mut i32 = &mut l;
    });
}

#[itest]
fn onready_multi_init() {
    expect_panic("init() on already initialized container fails", || {
        let mut l = OnReady::<i32>::new(|| 42);
        godot::private::auto_init(&mut l);
        godot::private::auto_init(&mut l);
    });
}

#[itest(skip)] // Not yet implemented.
fn onready_lifecycle_forget() {
    let mut forgetful = OnReadyWithImpl::create(false);
    let forgetful_copy = forgetful.clone();

    expect_panic(
        "Forgetting to initialize OnReady during ready() panics",
        move || {
            forgetful.notify(NodeNotification::Ready);
        },
    );

    forgetful_copy.free();
}

#[itest]
fn onready_lifecycle() {
    let mut obj = OnReadyWithImpl::create(true);

    obj.notify(NodeNotification::Ready);

    {
        let mut obj = obj.bind_mut();
        assert_eq!(*obj.auto, 11);
        assert_eq!(*obj.manual, 22);

        *obj.auto = 33;
        assert_eq!(*obj.auto, 33);
    }

    obj.free();
}

#[itest]
fn onready_lifecycle_without_impl() {
    let mut obj = OnReadyWithoutImpl::create();

    obj.notify(NodeNotification::Ready);

    {
        let mut obj = obj.bind_mut();
        assert_eq!(*obj.auto, 44);

        *obj.auto = 55;
        assert_eq!(*obj.auto, 55);
    }

    obj.free();
}

#[itest]
fn onready_lifecycle_with_impl_without_ready() {
    let mut obj = OnReadyWithImplWithoutReady::create();

    obj.notify(NodeNotification::Ready);

    {
        let mut obj = obj.bind_mut();
        assert_eq!(*obj.auto, 66);

        *obj.auto = 77;
        assert_eq!(*obj.auto, 77);
    }

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
struct OnReadyWithImpl {
    #[onready]
    auto: OnReady<i32>,
    #[onready]
    manual: OnReady<i32>,
    runs_manual_init: bool,
}

impl OnReadyWithImpl {
    fn create(runs_manual_init: bool) -> Gd<OnReadyWithImpl> {
        let obj = Self {
            auto: OnReady::new(|| 11),
            manual: OnReady::manual(),
            runs_manual_init,
        };

        Gd::from_object(obj)
    }
}

#[godot_api]
impl INode for OnReadyWithImpl {
    fn ready(&mut self) {
        assert_eq!(*self.auto, 11);

        if self.runs_manual_init {
            self.manual.init(22);
            assert_eq!(*self.manual, 22);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Class that doesn't have a #[godot_api] impl. Used to test whether variables are still initialized.
#[derive(GodotClass)]
#[class(base=Node)]
struct OnReadyWithoutImpl {
    #[onready]
    auto: OnReady<i32>,
    // No manual one, since those cannot be initialized without a ready() override.
    // (Technically they _can_ at the moment, but in the future we might ensure initialization after ready, so this is not a supported workflow).
}

// Rust-only impl, no proc macros.
impl OnReadyWithoutImpl {
    fn create() -> Gd<OnReadyWithoutImpl> {
        let obj = Self {
            auto: OnReady::new(|| 44),
        };

        Gd::from_object(obj)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Class that has a #[godot_api] impl, but does not override ready. Used to test whether variables are still initialized.
#[derive(GodotClass)]
#[class(base=Node)]
struct OnReadyWithImplWithoutReady {
    #[onready]
    auto: OnReady<i32>,
    // No manual one, since those cannot be initialized without a ready() override.
    // (Technically they _can_ at the moment, but in the future we might ensure initialization after ready, so this is not a supported workflow).
}

// Rust-only impl, no proc macros.
impl OnReadyWithImplWithoutReady {
    fn create() -> Gd<OnReadyWithImplWithoutReady> {
        let obj = Self {
            auto: OnReady::new(|| 66),
        };

        Gd::from_object(obj)
    }
}

#[godot_api]
impl INode for OnReadyWithoutImpl {
    // Declare another function to ensure virtual getter must be provided.
    fn process(&mut self, _delta: f64) {}
}
