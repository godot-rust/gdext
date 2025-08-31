/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::builtin::{vslice, GString, Signal, StringName};
use godot::classes::object::ConnectFlags;
use godot::classes::{Node, Node3D, Object, RefCounted};
use godot::meta::{FromGodot, GodotConvert, ToGodot};
use godot::obj::{Base, Gd, InstanceId, NewAlloc, NewGd};
use godot::prelude::ConvertError;
use godot::register::{godot_api, GodotClass};
use godot::sys;
use godot::sys::Global;

use crate::framework::itest;

#[itest]
fn signal_basic_connect_emit() {
    let mut emitter = Emitter::new_alloc();
    let receiver = Receiver::new_alloc();

    emitter.connect("signal_unit", &receiver.callable("receive_unit"));
    emitter.emit_signal("signal_unit", &[]);
    assert_eq!(receiver.bind().last_received(), LastReceived::Unit);

    emitter.connect("signal_int", &receiver.callable("receive_int"));
    emitter.emit_signal("signal_int", vslice![1278]);
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
#[itest]
fn signal_symbols_internal() {
    let mut emitter = Emitter::new_alloc();

    // Connect signals from inside.
    let tracker = Rc::new(Cell::new(0));
    let mut internal = emitter.bind_mut();
    internal.connect_signals_internal(tracker.clone());
    drop(internal);

    // Make sure that connection has been properly registered by Godot.
    assert!(!emitter.get_incoming_connections().is_empty());

    emitter.bind_mut().emit_signals_internal();

    // Check that closure is invoked.
    assert_eq!(tracker.get(), 1234, "Emit failed (closure)");

    // Check that instance methods self_receive() and self_receive_gd_inc1() are invoked.
    assert_eq!(
        emitter.bind().last_received_int,
        1234 + 1, // self_receive_gd_inc1() increments by 1, and should be called after self_receive().
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
#[itest]
fn signal_symbols_external() {
    let emitter = Emitter::new_alloc();
    let mut sig = emitter.signals().signal_int();

    // Local function; deliberately use a !Send type.
    let tracker = Rc::new(Cell::new(0));
    {
        let tracker = tracker.clone();
        sig.connect(move |i| {
            tracker.set(i);
        });
    }

    // Self-modifying method.
    sig.connect_self(Emitter::self_receive);

    // Connect to other object.
    let receiver = Receiver::new_alloc();
    sig.connect_other(&receiver, Receiver::receive_int_mut);

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
#[itest]
fn signal_symbols_complex_emit() {
    let emitter = Emitter::new_alloc();
    let arg = emitter.clone();
    let mut sig = emitter.signals().signal_obj();

    let tracker = Rc::new(RefCell::new(None));
    {
        let tracker = tracker.clone();
        sig.connect(move |obj: Gd<Object>, name: GString| {
            *tracker.borrow_mut() = Some((obj, name));
        });
    }

    // Forward compat: .upcast() here becomes a breaking change if we generalize AsArg to include derived->base conversions.
    sig.emit(&arg.upcast(), "hello");

    emitter.free();
}

#[itest]
fn signal_receiver_auto_disconnect() {
    let emitter = Emitter::new_alloc();
    let sig = emitter.signals().signal_int();

    let receiver = Receiver::new_alloc();
    sig.connect_other(&receiver, Receiver::receive_int_mut);

    let outgoing_connections = emitter.get_signal_connection_list("signal_int");
    let incoming_connections = receiver.get_incoming_connections();

    assert_eq!(incoming_connections.len(), 1);
    assert_eq!(incoming_connections, outgoing_connections);

    receiver.free();

    // Should be auto-disconnected by Godot.
    let outgoing_connections = emitter.get_signal_connection_list("signal_int");
    assert!(outgoing_connections.is_empty());
    emitter.free();
}

// "External" means connect/emit happens from outside the class, via Gd::signals().
#[itest]
fn signal_symbols_external_builder() {
    let emitter = Emitter::new_alloc();
    let mut sig = emitter.signals().signal_int();

    // Self-modifying method.
    sig.connect_self(Emitter::self_receive);

    // Connect to other object.
    let receiver_mut = Receiver::new_alloc();
    sig.builder()
        .name("receive_the_knowledge")
        .connect_other_mut(&receiver_mut, Receiver::receive_int_mut);

    sig.connect_other(&receiver_mut, Receiver::receive_int_mut);

    let tracker = Rc::new(Cell::new(0));
    {
        let tracker = tracker.clone();
        sig.builder().connect(move |i| tracker.set(i));
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
        receiver_mut.bind().last_received(),
        LastReceived::IntMut(552),
        "Emit failed (other object, mut method)"
    );

    // Check that closures set up with builder are invoked.
    assert_eq!(tracker.get(), 552, "Emit failed (builder local)");

    receiver_mut.free();
    emitter.free();
}

#[cfg(feature = "experimental-threads")]
#[itest]
fn signal_symbols_sync() {
    use std::sync::{Arc, Mutex};

    let emitter = Emitter::new_alloc();
    let mut sig = emitter.signals().signal_int();

    let sync_tracker = Arc::new(Mutex::new(0));
    {
        let sync_tracker = sync_tracker.clone();
        sig.builder()
            .connect_sync(move |i| *sync_tracker.lock().unwrap() = i);
    }

    sig.emit(1143);
    assert_eq!(
        *sync_tracker.lock().unwrap(),
        1143,
        "Emit failed (builder sync)"
    );

    emitter.free();
}

#[itest]
fn signal_symbols_engine(ctx: &crate::framework::TestContext) {
    // Add node to tree, to test Godot signal interactions.
    let mut node = Node::new_alloc();
    ctx.scene_tree.clone().add_child(&node);

    // API allows to only modify one signal at a time (borrowing &mut self).
    let renamed = node.signals().renamed();
    let renamed_count = Rc::new(Cell::new(0));
    {
        let renamed_count = renamed_count.clone();
        renamed.connect(move || renamed_count.set(renamed_count.get() + 1));
    }

    let entered = node.signals().child_entered_tree();
    let entered_tracker = Rc::new(RefCell::new(None));
    {
        let entered_tracker = entered_tracker.clone();

        entered.builder().connect(move |node| {
            *entered_tracker.borrow_mut() = Some(node);
        });
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

// Test that Node signals are accessible from a derived class.
#[itest]
fn signal_symbols_engine_inherited(ctx: &crate::framework::TestContext) {
    let mut node = Emitter::new_alloc();

    // Add to tree, so signals are propagated.
    ctx.scene_tree.clone().add_child(&node);

    let sig = node.signals().renamed();
    sig.connect_self(|this| {
        this.last_received_int = 887;
    });

    node.set_name("new name");

    assert_eq!(node.bind().last_received_int, 887);

    // Remove from tree for other tests.
    node.free();
}

// Test that Node signals are accessible from a derived class, with Node3D middleman.
#[itest]
fn signal_symbols_engine_inherited_indirect(ctx: &crate::framework::TestContext) {
    let original = Emitter::new_alloc();
    let mut node = original.clone().upcast::<Node3D>();

    // Add to tree, so signals are propagated.
    ctx.scene_tree.clone().add_child(&node);

    let sig = node.signals().renamed();
    sig.connect_other(&original, |this: &mut Emitter| {
        this.last_received_int = 887;
    });

    node.set_name("new name");

    assert_eq!(original.bind().last_received_int, 887);

    // Remove from tree for other tests.
    node.free();
}

// Test that Node signals are *internally* accessible from a derived class.
#[itest]
fn signal_symbols_engine_inherited_internal() {
    // No tree needed; signal is emitted manually.
    let mut node = Emitter::new_alloc();
    node.bind_mut().connect_base_signals_internal();
    node.bind_mut().emit_base_signals_internal();

    assert_eq!(node.bind().last_received_int, 553);
    node.free();
}

// Test that signal API methods accept engine types as receivers.
#[itest]
fn signal_symbols_connect_engine() {
    // No tree needed; signal is emitted manually.
    let node = Emitter::new_alloc();
    let mut engine = Node::new_alloc();
    engine.set_name("hello");

    node.signals()
        .property_list_changed()
        .connect_other(&engine, |this| {
            assert_eq!(this.get_name(), StringName::from("hello"));
        });

    node.signals()
        .property_list_changed()
        .builder()
        .connect_other_gd(&engine, |this| {
            assert_eq!(this.get_name(), StringName::from("hello"));
        });

    node.signals().property_list_changed().emit();

    node.free();
    engine.free();
}

// Test that rustc is capable of inferring the parameter types of closures passed to the signal API's connect methods.
#[itest]
fn signal_symbols_connect_inferred() {
    let user = Emitter::new_alloc();
    let engine = Node::new_alloc();

    // User signals.
    user.signals()
        .child_entered_tree()
        .connect_other(&engine, |this, mut child| {
            // Use methods that `Node` declares.
            let _ = this.get_path(); // ref.
            this.set_unique_name_in_owner(true); // mut.

            // `child` is also a node.
            let _ = child.get_path(); // ref.
            child.set_unique_name_in_owner(true); // mut.
        });

    user.signals().renamed().connect_self(|this| {
        // Use method/field that `Emitter` declares.
        this.connect_base_signals_internal();
        let _ = this.last_received_int;
    });

    // User signals, builder.
    user.signals().renamed().builder().connect_self_mut(|this| {
        // Use method/field that `Emitter` declares.
        this.connect_base_signals_internal();
        let _ = this.last_received_int;
    });

    // Engine signals.
    engine.signals().ready().connect_other(&user, |this| {
        // Use method/field that `Emitter` declares.
        this.connect_base_signals_internal();
        let _ = this.last_received_int;
    });

    // Engine signals, builder.
    engine
        .signals()
        .tree_exiting()
        .builder()
        .flags(ConnectFlags::DEFERRED)
        .connect_self_gd(|mut this| {
            // Use methods that `Node` declares.
            let _ = this.get_path(); // ref.
            this.set_unique_name_in_owner(true); // mut.
        });

    engine
        .signals()
        .tree_exiting()
        .builder()
        .connect_other_mut(&user, |this| {
            // Use methods that `Node` declares.
            use godot::obj::WithBaseField; // not recommended pattern; `*_gd()` connectors preferred.

            let _ = this.base().get_path(); // ref.
            this.base_mut().set_unique_name_in_owner(true); // mut.
        });

    engine
        .signals()
        .tree_exiting()
        .builder()
        .connect_other_gd(&user, |mut this| {
            // Use methods that `Node` declares.
            let _ = this.get_path(); // ref.
            this.set_unique_name_in_owner(true); // mut.
        });

    // Don't emit any signals, this test just needs to compile.

    user.free();
    engine.free();
}

// Test that Node signals are accessible from a derived class, when the class itself has no #[signal] declarations.
// Verifies the code path that only generates the traits, no dedicated signal collection.
#[itest]
fn signal_symbols_engine_inherited_no_own_signals() {
    let mut obj = Receiver::new_alloc();

    let sig = obj.signals().property_list_changed();
    sig.connect_self(|this| {
        this.receive_int(941);
    });

    obj.notify_property_list_changed();
    assert_eq!(obj.bind().last_received.get(), LastReceived::Int(941));

    obj.free();
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

#[itest]
fn enums_as_signal_args() {
    #[derive(Debug, Clone)]
    enum EventType {
        Ready,
    }

    impl GodotConvert for EventType {
        type Via = u8;
    }

    impl ToGodot for EventType {
        type Pass = godot::meta::ByValue;

        fn to_godot(&self) -> Self::Via {
            match self {
                EventType::Ready => 0,
            }
        }
    }

    impl FromGodot for EventType {
        fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
            match via {
                0 => Ok(Self::Ready),
                _ => Err(ConvertError::new("value out of range")),
            }
        }
    }

    #[derive(GodotClass)]
    #[class(base = RefCounted, init)]
    struct SignalObject {
        base: Base<RefCounted>,
    }

    #[godot_api]
    impl SignalObject {
        #[signal]
        fn game_event(ty: EventType);
    }

    let object = SignalObject::new_gd();
    let event = EventType::Ready;

    object.signals().game_event().emit(event);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helper types

/// Global sets the value of the received argument and whether it was a static function.
static LAST_STATIC_FUNCTION_ARG: Global<i64> = Global::default();

// Separate module to test signal visibility.
use emitter::Emitter;

mod emitter {
    use godot::obj::WithUserSignals;

    use super::*;

    #[derive(GodotClass)]
    #[class(init, base=Node3D)] // Node instead of Object to test some signals defined in superclasses.
    pub struct Emitter {
        _base: Base<Node3D>,
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
        pub(super) fn signal_obj(arg1: Gd<Object>, arg2: GString);

        #[func]
        pub fn self_receive(&mut self, arg1: i64) {
            self.last_received_int = arg1;
        }

        #[func]
        pub fn self_receive_gd_inc1(mut this: Gd<Self>, _arg1: i64) {
            this.bind_mut().last_received_int += 1;
        }

        #[func]
        pub fn self_receive_constant(&mut self) {
            self.last_received_int = 553;
        }

        #[func]
        fn self_receive_static(arg1: i64) {
            *LAST_STATIC_FUNCTION_ARG.lock() = arg1;
        }

        // "Internal" means connect/emit happens from within the class (via &mut self).

        pub fn connect_signals_internal(&mut self, tracker: Rc<Cell<i64>>) {
            let sig = self.signals().signal_int();
            sig.connect_self(Self::self_receive);
            sig.connect(Self::self_receive_static);
            sig.connect(move |i| tracker.set(i));
            sig.builder().connect_self_gd(Self::self_receive_gd_inc1);
        }

        pub fn emit_signals_internal(&mut self) {
            self.signals().signal_int().emit(1234);
        }

        pub fn connect_base_signals_internal(&mut self) {
            self.signals()
                .renamed()
                .connect_self(Emitter::self_receive_constant);
        }

        pub fn emit_base_signals_internal(&mut self) {
            self.signals().renamed().emit();
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
    // Do not declare any #[signal]s here -- explicitly test this implements WithSignal without them.

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

// Class which is deliberately `pub` but has only private `#[signal]` declaration.
// Regression test, as this caused "leaked private types" in the past.
#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct PubClassPrivSignal {
    _base: Base<Object>,
}

#[godot_api]
impl PubClassPrivSignal {
    #[signal]
    fn private_signal();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Custom callables

mod custom_callable {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use godot::builtin::{vslice, Callable, Signal};
    use godot::classes::Node;
    use godot::obj::{Gd, NewAlloc};

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
                node.emit_signal("test_signal", vslice![987i64]);
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
                node.emit_signal("test_signal", vslice![987i64]);
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
    // Custom callables - helper functions

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
