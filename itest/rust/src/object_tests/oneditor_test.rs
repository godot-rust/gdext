/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::{expect_panic, itest};
use godot::classes::notify::NodeNotification;
use godot::classes::{INode, Node, RefCounted};
use godot::register::{godot_api, GodotClass};

use godot::obj::{Gd, NewAlloc, OnEditor};

#[itest]
fn oneditor_deref() {
    let mut on_editor = OnEditor::new_invalid(0);
    on_editor.init(42);
    assert_eq!(*on_editor, 42);

    *on_editor = 44;
    assert_eq!(*on_editor, 44);
}

#[itest]
fn oneditor_no_value_panic_on_deref_primitive() {
    expect_panic("Deref on null fails for primitive", || {
        let on_editor_panic: OnEditor<i64> = OnEditor::new_invalid(0);
        let _ref: &i64 = &on_editor_panic;
    });
    expect_panic("Deref on null fails for Gd class", || {
        let on_editor_panic: OnEditor<Gd<RefCounted>> = OnEditor::default();
        let _ref: &Gd<RefCounted> = &on_editor_panic;
    });

    expect_panic("DerefMut on null fails for primitive", || {
        let mut on_editor_panic: OnEditor<i64> = OnEditor::new_invalid(0);
        let _ref: &mut i64 = &mut on_editor_panic;
    });
    expect_panic("DerefMut on null fails for Gd class", || {
        let mut on_editor_panic: OnEditor<Gd<RefCounted>> = OnEditor::default();
        let _ref: &mut Gd<RefCounted> = &mut on_editor_panic;
    });
}

#[itest]
fn oneditor_panic_on_ready() {
    let mut obj = OnEditorNoDefault::new_alloc();

    // causes the panic which is NOT propagated to godot-rust but prevents `ready` from being run.
    obj.notify(NodeNotification::READY);
    assert!(!obj.bind().was_ready_run);
    obj.free();
}

#[itest]
fn oneditor_no_panic_on_ready() {
    let mut obj = OnEditorNoDefault::new_alloc();
    obj.bind_mut().node_field.init(Node::new_alloc());
    obj.bind_mut().some_primitive.init(64);
    obj.notify(NodeNotification::READY);
    assert!(obj.bind().was_ready_run);
    obj.bind_mut().node_field.clone().free();
    obj.free();
}

#[derive(GodotClass)]
#[class(init, base=Node)]
struct OnEditorNoDefault {
    #[export]
    #[init(invalid = 0)]
    some_primitive: OnEditor<i64>,
    #[export]
    node_field: OnEditor<Gd<Node>>,

    /// Informs whether `ready()` has been run (false if a panic occurred).
    was_ready_run: bool,
}

#[godot_api]
impl INode for OnEditorNoDefault {
    fn ready(&mut self) {
        self.was_ready_run = true;
    }
}
