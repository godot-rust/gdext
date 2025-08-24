/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Integration of OnReady with #[init(load = "PATH")] is tested in save_load_test.rs.

use godot::classes::notify::NodeNotification;
use godot::classes::{INode, Node};
use godot::obj::{Gd, NewAlloc, OnReady};
use godot::prelude::{Base, ToGodot};
use godot::register::{godot_api, GodotClass};

use crate::framework::{expect_panic, itest};

#[itest]
fn onready_deref() {
    let node = Node::new_alloc();
    let mut l = OnReady::<i32>::new(|| 42);
    godot::private::auto_init(&mut l, &node);

    // DerefMut
    let mut_ref: &mut i32 = &mut l;
    assert_eq!(*mut_ref, 42);

    // Deref
    let l = l;
    let shared_ref: &i32 = &l;
    assert_eq!(*shared_ref, 42);

    node.free();
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
    let node = Node::new_alloc();

    expect_panic("init() on already initialized container fails", || {
        let mut l = OnReady::<i32>::new(|| 42);
        godot::private::auto_init(&mut l, &node);
        godot::private::auto_init(&mut l, &node);
    });

    node.free();
}

#[itest(skip)] // Not yet implemented.
fn onready_lifecycle_forget() {
    let mut forgetful = OnReadyWithImpl::create(false);
    let forgetful_copy = forgetful.clone();

    expect_panic(
        "Forgetting to initialize OnReady during ready() panics",
        move || {
            forgetful.notify(NodeNotification::READY);
        },
    );

    forgetful_copy.free();
}

#[itest]
fn onready_lifecycle() {
    let mut obj = OnReadyWithImpl::create(true);

    obj.notify(NodeNotification::READY);

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
fn onready_lifecycle_gd_self() {
    let mut obj = OnReadyWithImplGdSelf::create(true);

    obj.notify(NodeNotification::READY);

    {
        let mut obj = obj.bind_mut();
        assert_eq!(*obj.auto, 77);
        assert_eq!(*obj.manual, 55);

        *obj.auto = 44;
        assert_eq!(*obj.auto, 44);
    }

    obj.free();
}

#[itest]
fn onready_lifecycle_without_impl() {
    let mut obj = OnReadyWithoutImpl::create();

    obj.notify(NodeNotification::READY);

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
    let base = obj.clone().upcast::<Node>();

    obj.notify(NodeNotification::READY);

    {
        let mut obj = obj.bind_mut();
        assert_eq!(*obj.auto, 66);

        *obj.auto = 77;
        assert_eq!(*obj.auto, 77);

        // Test #[hint(no_onready)]: we can still initialize it (would panic if already auto-initialized).
        godot::private::auto_init(&mut obj.nothing, &base);
    }

    obj.free();
}

#[itest]
fn onready_property_access() {
    let mut obj = OnReadyWithImpl::create(true);
    obj.notify(NodeNotification::READY);

    obj.set("auto", &33.to_variant());
    obj.set("manual", &44.to_variant());

    {
        let obj = obj.bind();
        assert_eq!(*obj.auto, 33);
        assert_eq!(*obj.manual, 44);
    }

    let auto = obj.get("auto").to::<i32>();
    let manual = obj.get("manual").to::<i64>();
    assert_eq!(auto, 33);
    assert_eq!(manual, 44);

    obj.free();
}

#[itest]
fn init_attribute_node_key_lifecycle() {
    let mut obj = InitWithNodeOrBase::new_alloc();
    obj.set_name("CustomNodeName");

    let mut child = Node::new_alloc();
    child.set_name("child");
    obj.add_child(&child);

    obj.notify(NodeNotification::READY);

    {
        let obj = obj.bind();
        assert_eq!(obj.node.get_name(), "child".into());
        assert_eq!(obj.self_name.as_str(), "CustomNodeName");
    }

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(no_init, base=Node)]
struct OnReadyWithImpl {
    base: Base<Node>,
    #[var]
    auto: OnReady<i32>,
    #[var]
    manual: OnReady<i32>,
    runs_manual_init: bool,
}

impl OnReadyWithImpl {
    fn create(runs_manual_init: bool) -> Gd<OnReadyWithImpl> {
        Gd::from_init_fn(|base| Self {
            base,
            auto: OnReady::new(|| 11),
            manual: OnReady::manual(),
            runs_manual_init,
        })
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
#[class(no_init, base=Node)]
struct OnReadyWithoutImpl {
    base: Base<Node>,
    auto: OnReady<i32>,
    // No manual one, since those cannot be initialized without a ready() override.
    // (Technically they _can_ at the moment, but in the future we might ensure initialization after ready, so this is not a supported workflow).
}

// Rust-only impl, no proc macros.
impl OnReadyWithoutImpl {
    fn create() -> Gd<OnReadyWithoutImpl> {
        Gd::from_init_fn(|base| Self {
            base,
            auto: OnReady::new(|| 44),
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

type Ordy<T> = OnReady<T>;

// Class that has a #[godot_api] impl, but does not override ready. Used to test whether variables are still initialized.
#[derive(GodotClass)]
#[class(no_init, base=Node)]
struct OnReadyWithImplWithoutReady {
    base: Base<Node>,
    // Test also #[hint] at the same time.
    #[hint(onready)]
    auto: Ordy<i32>,
    // No manual one, since those cannot be initialized without a ready() override.
    // (Technically they _can_ at the moment, but in the future we might ensure initialization after ready, so this is not a supported workflow).
    #[hint(no_onready)]
    nothing: OnReady<i32>,
}

// Rust-only impl, no proc macros.
impl OnReadyWithImplWithoutReady {
    fn create() -> Gd<OnReadyWithImplWithoutReady> {
        Gd::from_init_fn(|base| Self {
            base,
            auto: Ordy::new(|| 66),
            nothing: Ordy::new(|| -111),
        })
    }
}

#[godot_api]
impl INode for OnReadyWithImplWithoutReady {
    // Declare another function to ensure virtual getter must be provided.
    fn process(&mut self, _delta: f64) {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Class that has overriden ready with Gd<Self> receiver. Used to test whether variables are still initialized.
#[derive(GodotClass)]
#[class(no_init, base=Node)]
struct OnReadyWithImplGdSelf {
    base: Base<Node>,
    #[hint(onready)]
    auto: OnReady<i32>,
    #[hint(no_onready)]
    manual: OnReady<i32>,
    runs_manual_init: bool,
}

impl OnReadyWithImplGdSelf {
    fn create(runs_manual_init: bool) -> Gd<OnReadyWithImplGdSelf> {
        Gd::from_init_fn(|base| Self {
            base,
            auto: OnReady::new(|| 77),
            manual: OnReady::manual(),
            runs_manual_init,
        })
    }
}

#[godot_api]
impl INode for OnReadyWithImplGdSelf {
    #[func(gd_self)]
    fn ready(mut this: Gd<Self>) {
        let mut this_bind = this.bind_mut();
        assert_eq!(*this_bind.auto, 77);

        if this_bind.runs_manual_init {
            this_bind.manual.init(55);
            assert_eq!(*this_bind.manual, 55);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// #[init(node = "NodePath")] Attribute
// Used to test whether `node` keys sets variables' expected values.
#[derive(GodotClass)]
#[class(init, base = Node)]
struct InitWithNodeOrBase {
    base: Base<Node>,
    #[init(node = "child")]
    node: OnReady<Gd<Node>>,
    #[init(val = OnReady::from_base_fn(|b| b.get_name().to_string()))]
    self_name: OnReady<String>,
}

#[godot_api]
impl INode for InitWithNodeOrBase {
    fn ready(&mut self) {
        assert_eq!(self.node.get_name(), "child".into());
        assert_eq!(self.self_name.as_str(), "CustomNodeName");
    }
}
