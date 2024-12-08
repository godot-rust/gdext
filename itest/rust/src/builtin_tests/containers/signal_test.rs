/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{GString, Signal, StringName};
use godot::classes::{Object, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, NewAlloc, NewGd, WithBaseField};
use godot::register::{godot_api, GodotClass};
use godot::sys;
use std::cell::Cell;

use crate::framework::itest;

#[itest]
fn signal_basic_connect_emit() {
    let mut emitter = Emitter::new_alloc();
    let receiver = Receiver::new_alloc();

    let args = [
        vec![],
        vec![987.to_variant()],
        vec![receiver.to_variant(), SIGNAL_ARG_STRING.to_variant()],
    ];

    for (i, arg) in args.iter().enumerate() {
        let signal_name = format!("emitter_{i}");
        let receiver_name = format!("receiver_{i}");

        emitter.connect(&signal_name, &receiver.callable(&receiver_name));
        emitter.emit_signal(&signal_name, arg);

        assert!(receiver.bind().used[i].get());
    }

    receiver.free();
    emitter.free();
}

#[itest]
fn signal_construction_and_id() {
    let mut object = RefCounted::new_gd();
    let object_id = object.instance_id();

    object.add_user_signal("test_signal");

    let signal = Signal::from_object_signal(&object, "test_signal");

    assert!(!signal.is_null());
    assert_eq!(signal.name(), StringName::from("test_signal"));
    assert_eq!(signal.object(), Some(object.clone().upcast()));
    assert_eq!(signal.object_id(), Some(object_id));

    // Invalidating the object still returns the old ID, however not the object.
    drop(object);
    assert_eq!(signal.object_id(), Some(object_id));
    assert_eq!(signal.object(), None);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helper types

#[derive(GodotClass)]
#[class(init, base=Object)]
struct Emitter {}

#[godot_api]
impl Emitter {
    #[signal]
    fn emitter_0();

    #[signal]
    fn emitter_1(arg1: i64);

    #[signal]
    fn emitter_2(arg1: Gd<Object>, arg2: GString);
}

#[derive(GodotClass)]
#[class(init, base=Object)]
struct Receiver {
    used: [Cell<bool>; 3],
    base: Base<Object>,
}

#[godot_api]
impl Receiver {
    #[func]
    fn receiver_0(&self) {
        self.used[0].set(true);
    }

    #[func]
    fn receiver_1(&self, arg1: i64) {
        self.used[1].set(true);
        assert_eq!(arg1, 987);
    }

    #[func]
    fn receiver_2(&self, arg1: Gd<Object>, arg2: GString) {
        assert_eq!(self.base().clone(), arg1);
        assert_eq!(SIGNAL_ARG_STRING, arg2.to_string());

        self.used[2].set(true);
    }
}

const SIGNAL_ARG_STRING: &str = "Signal string arg";

// ----------------------------------------------------------------------------------------------------------------------------------------------
// 4.2+ custom callables

#[cfg(since_api = "4.2")]
mod custom_callable {
    use godot::builtin::{Callable, Signal};
    use godot::classes::Node;
    use godot::meta::ToGodot;
    use godot::obj::{Gd, NewAlloc};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use crate::builtin_tests::containers::callable_test::custom_callable::PanicCallable;
    use crate::framework::{itest, TestContext};

    #[itest]
    fn signal_panic_user_from_fn() {
        connect_signal_panic_shared(
            "test_signal",
            connect_signal_panic_from_fn,
            |node| {
                node.add_user_signal("test_signal");
            },
            |node| {
                node.emit_signal("test_signal", &[987i64.to_variant()]);
            },
        );
    }

    #[itest]
    fn signal_panic_user_from_custom() {
        connect_signal_panic_shared(
            "test_signal",
            connect_signal_panic_from_custom,
            |node| {
                node.add_user_signal("test_signal");
            },
            |node| {
                node.emit_signal("test_signal", &[987i64.to_variant()]);
            },
        );
    }

    #[itest]
    fn signal_panic_from_fn_tree_entered(ctx: &TestContext) {
        connect_signal_panic_shared(
            "tree_entered",
            connect_signal_panic_from_fn,
            |_node| {},
            |node| add_remove_child(ctx, node),
        );
    }

    #[itest]
    fn signal_panic_from_custom_tree_entered(ctx: &TestContext) {
        connect_signal_panic_shared(
            "tree_entered",
            connect_signal_panic_from_custom,
            |_node| {},
            |node| add_remove_child(ctx, node),
        );
    }

    #[itest]
    fn signal_panic_from_fn_tree_exiting(ctx: &TestContext) {
        connect_signal_panic_shared(
            "tree_exiting",
            connect_signal_panic_from_fn,
            |_node| {},
            |node| add_remove_child(ctx, node),
        );
    }

    #[itest]
    fn connect_signal_panic_from_custom_tree_exiting(ctx: &TestContext) {
        connect_signal_panic_shared(
            "tree_exiting",
            connect_signal_panic_from_custom,
            |_node| {},
            |node| add_remove_child(ctx, node),
        );
    }

    #[itest]
    fn signal_panic_from_fn_tree_exited(ctx: &TestContext) {
        connect_signal_panic_shared(
            "tree_exited",
            connect_signal_panic_from_fn,
            |_node| {},
            |node| add_remove_child(ctx, node),
        );
    }

    #[itest]
    fn signal_panic_from_custom_tree_exited(ctx: &TestContext) {
        connect_signal_panic_shared(
            "tree_exited",
            connect_signal_panic_from_custom,
            |_node| {},
            |node| {
                ctx.scene_tree.clone().add_child(&*node);
                ctx.scene_tree.clone().remove_child(&*node);
            },
        );
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // 4.2+ custom callables - helper functions

    fn add_remove_child(ctx: &TestContext, node: &mut Gd<Node>) {
        let mut tree = ctx.scene_tree.clone();
        tree.add_child(&*node);
        tree.remove_child(&*node);
    }

    fn connect_signal_panic_shared(
        signal: &str,
        callable: impl FnOnce(Arc<AtomicU32>) -> Callable,
        before: impl FnOnce(&mut Gd<Node>),
        emit: impl FnOnce(&mut Gd<Node>),
    ) {
        let mut node = Node::new_alloc();
        before(&mut node);

        let signal = Signal::from_object_signal(&node, signal);

        let received = Arc::new(AtomicU32::new(0));
        let callable = callable(received.clone());
        signal.connect(&callable, 0);

        emit(&mut node);
        assert_eq!(1, received.load(Ordering::SeqCst));

        node.free();
    }

    fn connect_signal_panic_from_fn(received: Arc<AtomicU32>) -> Callable {
        Callable::from_local_fn("test", move |_args| {
            panic!("TEST: {}", received.fetch_add(1, Ordering::SeqCst))
        })
    }

    fn connect_signal_panic_from_custom(received: Arc<AtomicU32>) -> Callable {
        Callable::from_custom(PanicCallable(received))
    }
}
