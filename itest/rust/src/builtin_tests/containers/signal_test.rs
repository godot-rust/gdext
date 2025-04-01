/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::builtin::{GString, Signal, StringName};
use godot::classes::{Node, Object, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, InstanceId, NewAlloc, NewGd};
use godot::register::{godot_api, GodotClass};
use godot::sys;
use godot::sys::Global;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[itest]
fn signal_basic_connect_emit() {
    let mut emitter = Emitter::new_alloc();
    let receiver = Receiver::new_alloc();

    emitter.connect("signal_unit", &receiver.callable("receive_unit"));
    emitter.emit_signal("signal_unit", &[]);
    assert_eq!(receiver.bind().last_received(), LastReceived::Unit);

    emitter.connect("signal_int", &receiver.callable("receive_int"));
    emitter.emit_signal("signal_int", &[1278.to_variant()]);
    assert_eq!(receiver.bind().last_received(), LastReceived::Int(1278));

    let emitter_variant = emitter.to_variant();
    emitter.connect("signal_obj", &receiver.callable("receive_obj"));
    emitter.emit_signal("signal_obj", &[emitter_variant]);
    assert_eq!(
        receiver.bind().last_received(),
        LastReceived::Object(emitter.instance_id())
    );

    receiver.free();
    emitter.free();
}

// "Internal" means connect/emit happens from within the class, via self.signals().
#[cfg(since_api = "4.2")]
#[itest]
fn signal_symbols_internal() {
    let mut emitter = Emitter::new_alloc();

    // Connect signals from inside.
    let tracker = Rc::new(Cell::new(0));
    let mut internal = emitter.bind_mut();
    internal.connect_signals_internal(tracker.clone());
    drop(internal);

    emitter.bind_mut().emit_signals_internal();

    // Check that closure is invoked.
    assert_eq!(tracker.get(), 1234, "Emit failed (closure)");

    // Check that instance method is invoked.
    assert_eq!(
        emitter.bind().last_received_int,
        1234,
        "Emit failed (method)"
    );

    // Check that static function is invoked.
    assert_eq!(
        *LAST_STATIC_FUNCTION_ARG.lock(),
        1234,
        "Emit failed (static function)"
    );

    emitter.free();
}

// "External" means connect/emit happens from outside the class, via Gd::signals().
#[cfg(since_api = "4.2")]
#[itest]
fn signal_symbols_external() {
    let mut emitter = Emitter::new_alloc();
    let mut sig = emitter.signals().signal_int();

    // Local function; deliberately use a !Send type.
    let tracker = Rc::new(Cell::new(0));
    {
        let tracker = tracker.clone();
        sig.connect_g(move |i| {
            tracker.set(i);
        });
    }

    // Self-modifying method.
    sig.connect_self(Emitter::self_receive);

    // Connect to other object.
    let receiver = Receiver::new_alloc();
    sig.connect(&receiver, Receiver::receive_int_mut);

    // Emit signal (now via tuple).
    sig.emit_tuple((987,));

    // Check that closure is invoked.
    assert_eq!(tracker.get(), 987, "Emit failed (closure)");

    // Check that instance method is invoked.
    assert_eq!(
        emitter.bind().last_received_int,
        987,
        "Emit failed (method)"
    );

    // Check that *other* instance method is invoked.
    assert_eq!(
        receiver.bind().last_received(),
        LastReceived::IntMut(987),
        "Emit failed (other object method)"
    );

    receiver.free();
    emitter.free();
}

// "External" means connect/emit happens from outside the class, via Gd::signals().
#[cfg(since_api = "4.2")]
#[itest]
fn signal_symbols_external_builder() {
    let mut emitter = Emitter::new_alloc();
    let mut sig = emitter.signals().signal_int();

    // Self-modifying method.
    sig.connect_builder()
        .object_self()
        .method_mut(Emitter::self_receive)
        .done();

    // Connect to other object.
    let receiver_mut = Receiver::new_alloc();
    sig.connect_builder()
        .object(&receiver_mut)
        .method_mut(Receiver::receive_int_mut)
        .done();

    // Connect to yet another object, immutable receiver.
    let receiver_immut = Receiver::new_alloc();
    sig.connect_builder()
        .object(&receiver_immut)
        .method_immut(Receiver::receive_int)
        .done();

    let tracker = Rc::new(Cell::new(0));
    {
        let tracker = tracker.clone();
        sig.connect_builder()
            .function(move |i| tracker.set(i))
            .done();
    }

    // Emit signal.
    sig.emit(552);

    // Check that closure is invoked.
    assert_eq!(tracker.get(), 552, "Emit failed (closure)");

    // Check that self instance method (mut) is invoked.
    assert_eq!(
        emitter.bind().last_received_int,
        552,
        "Emit failed (mut method)"
    );

    // Check that *other* instance method is invoked.
    assert_eq!(
        receiver_immut.bind().last_received(),
        LastReceived::Int(552),
        "Emit failed (other object, immut method)"
    );

    // Check that *other* instance method is invoked.
    assert_eq!(
        receiver_mut.bind().last_received(),
        LastReceived::IntMut(552),
        "Emit failed (other object, mut method)"
    );

    // Check that closures set up with builder are invoked.
    assert_eq!(tracker.get(), 552, "Emit failed (builder local)");

    receiver_immut.free();
    receiver_mut.free();
    emitter.free();
}

#[cfg(all(since_api = "4.2", feature = "experimental-threads"))]
#[itest]
fn signal_symbols_sync() {
    use std::sync::{Arc, Mutex};

    let mut emitter = Emitter::new_alloc();
    let mut sig = emitter.signals().signal_int();

    let sync_tracker = Arc::new(Mutex::new(0));
    {
        let sync_tracker = sync_tracker.clone();
        sig.connect_builder()
            .function(move |i| *sync_tracker.lock().unwrap() = i)
            .sync()
            .done();
    }

    sig.emit(1143);
    assert_eq!(
        *sync_tracker.lock().unwrap(),
        1143,
        "Emit failed (builder sync)"
    );

    emitter.free();
}

#[cfg(since_api = "4.2")]
#[itest]
fn signal_symbols_engine(ctx: &crate::framework::TestContext) {
    // Add node to tree, to test Godot signal interactions.
    let mut node = Node::new_alloc();
    ctx.scene_tree.clone().add_child(&node);

    // Deliberately declare here, because there was a bug with wrong lifetime, which would not compile due to early-dropped temporary.
    let mut signals_in_node = node.signals();
    let mut renamed = signals_in_node.renamed();
    let mut entered = signals_in_node.child_entered_tree();

    let renamed_count = Rc::new(Cell::new(0));
    let entered_tracker = Rc::new(RefCell::new(None));
    {
        let renamed_count = renamed_count.clone();
        let entered_tracker = entered_tracker.clone();

        entered
            .connect_builder()
            .function(move |node| {
                *entered_tracker.borrow_mut() = Some(node);
            })
            .done();

        renamed.connect_g(move || renamed_count.set(renamed_count.get() + 1));
    }

    // Apply changes, triggering signals.
    node.set_name("new name");
    let child = Node::new_alloc();
    node.add_child(&child);

    // Verify that signals were emitted.
    let entered_node = entered_tracker.take();
    assert_eq!(renamed_count.get(), 1, "Emit failed: Node::renamed");
    assert_eq!(
        entered_node,
        Some(child),
        "Emit failed: Node::child_entered_tree"
    );

    // Manually emit a signal a 2nd time.
    node.signals().renamed().emit();
    assert_eq!(renamed_count.get(), 2, "Manual emit failed: Node::renamed");

    // Remove from tree for other tests.
    node.free();
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

/// Global sets the value of the received argument and whether it was a static function.
static LAST_STATIC_FUNCTION_ARG: Global<i64> = Global::default();

// Separate module to test signal visibility.
use emitter::Emitter;

mod emitter {
    use super::*;
    use godot::obj::WithUserSignals;

    #[derive(GodotClass)]
    #[class(init, base=Object)]
    pub struct Emitter {
        _base: Base<Object>,
        #[cfg(since_api = "4.2")]
        pub last_received_int: i64,
    }

    #[godot_api]
    impl Emitter {
        #[signal]
        fn signal_unit();

        // Public to demonstrate usage inside module.
        #[signal]
        pub fn signal_int(arg1: i64);

        #[signal]
        fn signal_obj(arg1: Gd<Object>, arg2: GString);

        #[func]
        pub fn self_receive(&mut self, arg1: i64) {
            #[cfg(since_api = "4.2")]
            {
                self.last_received_int = arg1;
            }
        }

        #[func]
        fn self_receive_static(arg1: i64) {
            *LAST_STATIC_FUNCTION_ARG.lock() = arg1;
        }

        // "Internal" means connect/emit happens from within the class (via &mut self).

        #[cfg(since_api = "4.2")]
        pub fn connect_signals_internal(&mut self, tracker: Rc<Cell<i64>>) {
            let mut sig = self.signals().signal_int();
            sig.connect_self(Self::self_receive);
            sig.connect_g(Self::self_receive_static);
            sig.connect_g(move |i| tracker.set(i));
        }

        #[cfg(since_api = "4.2")]
        pub fn emit_signals_internal(&mut self) {
            self.signals().signal_int().emit(1234);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
enum LastReceived {
    #[default]
    Nothing,
    Unit,
    Int(i64),
    IntMut(i64),
    Object(InstanceId),
}

#[derive(GodotClass)]
#[class(init, base=Object)]
struct Receiver {
    last_received: Cell<LastReceived>,
    base: Base<Object>,
}

#[godot_api]
impl Receiver {
    fn last_received(&self) -> LastReceived {
        self.last_received.get()
    }

    // Note: asserting inside #[func] will be caught by FFI layer and not cause a call-site panic, thus not fail the test.
    // Therefore, store received values and check them manually in the test.

    #[func]
    fn receive_unit(&self) {
        self.last_received.set(LastReceived::Unit);
    }

    #[func]
    fn receive_int(&self, arg1: i64) {
        self.last_received.set(LastReceived::Int(arg1));
    }

    fn receive_int_mut(&mut self, arg1: i64) {
        self.last_received.set(LastReceived::IntMut(arg1));
    }

    #[func]
    fn receive_obj(&self, obj: Gd<Object>) {
        self.last_received
            .set(LastReceived::Object(obj.instance_id()));
    }
}

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
