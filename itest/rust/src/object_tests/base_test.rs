/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::{expect_panic, itest};

#[itest(skip)]
fn base_test_is_weak() {
    // TODO check that Base is a weak pointer (doesn't keep the object alive)
    // This might not be needed, as we have leak detection, but it could highlight regressions faster
}

#[itest]
fn base_access_unbound() {
    let mut obj = Based::new_alloc();

    let pos = Vector2::new(-5.5, 7.0);
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    obj.free();
}

// Tests whether access to base is possible from outside the Gd<T>, even if there is no Base<T> field.
#[itest]
fn base_access_unbound_no_field() {
    let mut obj = Baseless::new_alloc();

    let pos = Vector2::new(-5.5, 7.0);
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    obj.free();
}

#[itest]
fn base_display() {
    let obj = Based::new_alloc();
    {
        let guard = obj.bind();
        let id = guard.base().instance_id();

        // We expect the dynamic type to be part of Godot's to_string(), so Based and not Node2D
        let actual = format!(".:{}:.", guard.base);
        let expected = format!(".:<Based#{id}>:.");

        assert_eq!(actual, expected);
    }
    obj.free();
}

#[itest]
fn base_debug() {
    let obj = Based::new_alloc();
    {
        let guard = obj.bind();
        let id = guard.base().instance_id();

        // We expect the dynamic type to be part of Godot's to_string(), so Based and not Node2D
        let actual = format!(".:{:?}:.", guard.base);
        let expected = format!(".:Base {{ id: {id}, class: Based }}:.");

        assert_eq!(actual, expected);
    }
    obj.free();
}

// Compatibility check until v0.4 Base::to_gd() is removed.
#[itest]
fn base_with_init() {
    let obj = Gd::<Based>::from_init_fn(|base| {
        #[allow(deprecated)]
        base.to_gd().set_rotation(11.0);
        Based { base, i: 732 }
    });

    {
        let guard = obj.bind();
        assert_eq!(guard.i, 732);
        assert_eq!(guard.base().get_rotation(), 11.0);
    }
    obj.free();
}

#[itest]
fn base_gd_self() {
    let obj = Based::new_alloc();
    let obj2 = obj.bind().access_gd_self();

    assert_eq!(obj, obj2);
    assert_eq!(obj.instance_id(), obj2.instance_id());

    obj.free();
}

// Hardening against https://github.com/godot-rust/gdext/issues/711.
#[itest]
fn base_smuggling() {
    let (mut obj, extracted_base) = create_object_with_extracted_base();

    // This works because Gd<T> additionally stores the instance ID (through cached_rtti).
    let extracted_base_obj = extracted_base.__constructed_gd();
    assert_eq!(extracted_base_obj.instance_id(), obj.instance_id());

    // This _also_ works because Gd<T> has the direct object pointer to the Godot object.
    obj.set_position(Vector2::new(1.0, 2.0));
    assert_eq!(extracted_base_obj.get_position(), Vector2::new(1.0, 2.0));

    // Destroy base externally.
    extracted_base_obj.free();

    // Access to object should now fail.
    expect_panic("object with dead base: calling base methods", || {
        obj.get_position();
    });
    expect_panic("object with dead base: bind()", || {
        obj.bind();
    });
    expect_panic("object with dead base: instance_id()", || {
        obj.instance_id();
    });
    expect_panic("object with dead base: clone()", || {
        let _ = obj.clone();
    });
    expect_panic("object with dead base: upcast()", || {
        obj.upcast::<Object>();
    });

    // Now vice versa: destroy object, access base.
    let (obj, extracted_base) = create_object_with_extracted_base();
    obj.free();

    expect_panic("accessing extracted base of dead object", || {
        extracted_base.__constructed_gd().get_position();
    });
}

// While base swapping isn't an encouraged workflow, it can also be regarded as a quicker way to swap all individual properties of two base
// objects -- which is also allowed. It's also similar to slicing in C++. So this is a Ship-of-Theseus problem, and we don't install ergonomic
// obstacles to prevent it. Here, we test that results are expected and safe.
#[itest]
fn base_swapping() {
    let (one, mut one_ext_base) = create_object_with_extracted_base();
    let one_id = one.instance_id();

    let mut two = Based::new_alloc();
    let two_id = two.instance_id();

    std::mem::swap(&mut one_ext_base, &mut two.bind_mut().base);

    // Gd<T> itself isn't affected (it stores the ID separately).
    assert_eq!(one_id, one.instance_id());
    assert_eq!(two_id, two.instance_id());

    // However, the base now has the other object's ID. Gd<T> and T.base having distinct IDs is a bit unintuitive and could lead to follow-up
    // logic errors. One option to prevent this would be to add a base integrity check on the entire Gd<T> API (it can't be done from the
    // Base<T> side, since that only has direct access to the object pointer, while Gd<T> has access to the object pointer _and_ the base field).
    // Not sure if this is worth the effort + complexity though, given that it almost requires malice to get into such a situation.
    assert_eq!(one.instance_id(), two.bind().base().instance_id());
    assert_eq!(
        two.instance_id(),
        one_ext_base.__constructed_gd().instance_id()
    );

    one.free();
    two.free();
}

fn create_object_with_extracted_base() -> (Gd<Baseless>, Base<Node2D>) {
    let mut extracted_base = None;
    let obj = Baseless::smuggle_out(&mut extracted_base);
    let extracted_base = extracted_base.expect("smuggling didn't work");

    (obj, extracted_base)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
pub struct RefBase {
    pub base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for RefBase {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

// Also used in base_init_test.rs.
pub(super) use renamed_bases::Based;

mod renamed_bases {
    use godot::classes::INode2D;
    use godot::prelude::godot_api;

    use super::{GodotClass, Node2D};

    // Test #[hint].
    type Super<T> = super::Base<T>;
    type Base<T> = T;

    #[derive(GodotClass)]
    #[class(base = Node2D)]
    pub struct Based {
        #[hint(base)]
        pub base: Super<Node2D>, // de-facto: Base<Node2D>.

        // This can coexist because it's not really a base.
        #[hint(no_base)]
        pub i: Base<i32>, // de-facto: i32
    }

    #[godot_api]
    impl INode2D for Based {
        fn init(base: godot::obj::Base<Self::Base>) -> Self {
            Based { base, i: 0 }
        }
    }
}

impl Based {
    fn access_gd_self(&self) -> Gd<Self> {
        use godot::obj::WithBaseField as _;
        self.to_gd()
    }
}

#[derive(GodotClass)]
#[class(init, base=Node2D)]
struct Baseless {
    // No need for fields, we just test if we can access this as Gd<Node2D>.
}

impl Baseless {
    /// Steals the `Base<T>` from a newly constructed object and stores it in the output parameter.
    fn smuggle_out(other_base: &mut Option<Base<Node2D>>) -> Gd<Self> {
        Gd::from_init_fn(|base| {
            *other_base = Some(base);
            Self {}
        })
    }
}

#[derive(GodotClass)]
#[class] // <- also test this syntax.
pub(super) struct RefcBased {
    // pub(super): also used in base_init_test.rs.
    pub base: Base<RefCounted>,
}

// Only needed in base_init_test.rs.
#[godot_api]
impl IRefCounted for RefcBased {
    fn init(base: Base<RefCounted>) -> Self {
        #[cfg(since_api = "4.2")]
        base.to_init_gd(); // Immediately dropped.
        Self { base }
    }
}

// Only needed in base_init_test.rs.
#[cfg(since_api = "4.2")]
#[godot_api(no_typed_signals)]
impl RefcBased {
    /// Used in `base_init_test.rs` to test that a base pointer can be extracted during initialization.
    pub fn split_simple() -> (Gd<Self>, Gd<RefCounted>) {
        let mut moved_out = None;

        let self_gd = Gd::from_init_fn(|base| {
            moved_out = Some(base.to_init_gd()); // Moved out.
            Self { base }
        });

        (self_gd, moved_out.unwrap())
    }

    /// Used in `base_init_test.rs`, testing extraction + several drops happening at different times.
    pub fn split_intermixed() -> (Gd<Self>, Gd<RefCounted>) {
        let mut moved_out = None;

        let self_gd = Gd::from_init_fn(|base| {
            let gd = base.to_init_gd(); // Explicitly dropped below.

            drop(base.to_init_gd()); // Immediately dropped.

            let _local_copy = base.to_init_gd(); // Dropped at end of scope.
            moved_out = Some(base.to_init_gd()); // Moved out.

            drop(gd);

            Self { base }
        });

        (self_gd, moved_out.unwrap())
    }
}
