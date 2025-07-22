/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::ClassDb;
use crate::framework::{expect_panic, itest};
use godot::prelude::*;

#[itest(skip)]
fn base_test_is_weak() {
    // TODO check that Base is a weak pointer (doesn't keep the object alive)
    // This might not be needed, as we have leak detection, but it could highlight regressions faster
}

#[itest]
fn base_instance_id() {
    let obj = Based::new_alloc();
    let _obj_id = dbg!(obj.instance_id());
    //obj.call("unreference", &[]);
    obj.free();
}

// #[itest(focus)]
#[itest]
fn base_instance_id2() {
    {
        let obj = RefBase::new_gd();
        let _obj_id = dbg!(obj.instance_id());
        // obj.call("unreference", &[]);

        eprintln!("--------------------------");
    }
    eprintln!("--end fn------------------");
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
fn base_during_init() {
    let obj = Gd::<Based>::from_init_fn(|mut base| {
        // Test both temporary + local-variable syntax.
        base.as_init_gd().set_rotation(22.0);

        let gd = base.as_init_gd();
        gd.set_position(Vector2::new(100.0, 200.0));

        Based { base, i: 456 }
    });

    let guard = obj.bind();
    assert_eq!(guard.i, 456);
    assert_eq!(guard.base().get_rotation(), 22.0);
    assert_eq!(guard.base().get_position(), Vector2::new(100.0, 200.0));
    drop(guard);

    obj.free();
}

// This isn't recommended, but test what happens if someone clones and stores the Gd<T>.
#[itest]
fn base_during_init_extracted_gd() {
    let mut extractor = None;

    let obj = Gd::<Based>::from_init_fn(|mut base| {
        extractor = Some(base.as_init_gd().clone());

        Based { base, i: 456 }
    });

    let extracted = extractor.expect("extraction failed");
    assert_eq!(extracted.instance_id(), obj.instance_id());
    assert_eq!(extracted, obj.clone().upcast());

    // Destroy through the extracted Gd<T>.
    extracted.free();
    assert!(
        !obj.is_instance_valid(),
        "object should be invalid after base ptr is freed"
    );
}

// Checks bad practice of rug-pulling the base pointer.
#[itest]
fn base_during_init_freed_gd() {
    let mut free_executed = false;

    expect_panic("base object is destroyed", || {
        let _obj = Gd::<Based>::from_init_fn(|mut base| {
            let obj = base.as_init_gd().clone();
            obj.free(); // Causes the problem, but doesn't panic yet.
            free_executed = true;

            Based { base, i: 456 }
        });
    });

    assert!(
        free_executed,
        "free() itself doesn't panic, but following construction does"
    );
}

#[itest]
fn base_during_init_refcounted_simple() {
    {
        let obj = Gd::from_init_fn(|base| {
            eprintln!("---- before to_init_gd() ----");
            base.to_init_gd(); // Immediately dropped.
            eprintln!("---- after to_init_gd() ----");

            RefcBased { base }
        });
        eprintln!("After construction: refc={}", obj.get_reference_count());
    }

    // let mut last = Gd::<RefCounted>::from_instance_id(InstanceId::from_i64(-9223372001555511512));
    // last.call("unreference", &[]);
}

// Tests that the auto-decrement of surplus references also works when instantiated through the engine.
#[itest(focus)]
fn base_during_init_refcounted_from_engine() {
    let db = ClassDb::singleton();
    let obj = db.instantiate("RefcBased");

    // let mut last = Gd::<RefCounted>::from_instance_id(InstanceId::from_i64(-9223372001555511512));
    // last.call("unreference", &[]);
}

// #[itest(focus)]
#[itest]
fn base_during_init_refcounted() {
    let obj = RefcBased::new_gd();

    println!("After construction: refc={}", obj.get_reference_count());
    // obj.call("unreference", &[]);
    //
    // println!("After dec-ref: refc={}", obj.get_reference_count());
}

// #[itest(focus)]
// fn refcounted_drop() {
//     let a = RefCounted::new_gd();
//     let b = a.clone();
//     a.clone();
//     let c = b.clone();
//     drop(b);
//
//     assert_eq!(a.get_reference_count(), 2);
// }

#[itest]
fn base_during_init_refcounted_2() {
    // Instantiate with multiple Gd<T> references.
    let (obj, mut base) = RefcBased::with_split();
    let id = obj.instance_id();
    dbg!(&id);
    dbg!(id.to_i64() as u64);
    dbg!(base.instance_id().to_i64() as u64);

    // base.call("unreference", &[]);
    base.call("unreference", &[]);

    assert_eq!(obj.instance_id(), base.instance_id());
    assert_eq!(base.get_reference_count(), 2);
    assert_eq!(obj.get_reference_count(), 2);

    drop(base);
    assert_eq!(obj.get_reference_count(), 1);
    assert_eq!(obj.get_reference_count(), 1);
    drop(obj);

    assert!(!id.lookup_validity(), "last drop destroyed the object");
}

#[cfg(debug_assertions)]
#[itest]
fn base_during_init_outside_init() {
    let mut obj = Based::new_alloc();

    expect_panic("as_init_gd() outside init() function", || {
        let mut guard = obj.bind_mut();
        let _gd = guard.base.as_init_gd(); // Panics in Debug builds.
    });

    obj.free();
}

#[cfg(debug_assertions)]
#[itest]
fn base_during_init_to_gd() {
    expect_panic("WithBaseField::to_gd() inside init() function", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let temp_obj = Based { base, i: 999 };

            // This should panic because we're calling to_gd() during initialization
            let _gd = godot::obj::WithBaseField::to_gd(&temp_obj);

            temp_obj
        });
    });
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
        // dbg!(base.to_gd());
        Self { base }
    }
}

use renamed_bases::Based;
mod renamed_bases {
    use super::{GodotClass, Node2D};
    use godot::classes::INode2D;
    use godot::prelude::godot_api;

    // Test #[hint].
    type Super<T> = super::Base<T>;
    type Base<T> = T;

    #[derive(GodotClass)]
    #[class( base = Node2D)]
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
            // dbg!(base.to_gd());
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
struct RefcBased {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for RefcBased {
    // fn init(mut base: Base<RefCounted>) -> Self {
    //     println!(
    //         "Before to_init_gd(): refc={}",
    //         base.as_init_gd().get_reference_count()
    //     );
    //     let copy = base.to_init_gd();
    //     println!("Inside init(): refc={}", copy.get_reference_count());
    //     drop(copy);
    //     println!(
    //         "After to_init_gd(): refc={}",
    //         base.as_init_gd().get_reference_count()
    //     );
    //
    //     Self { base }
    // }
    fn init(base: Base<RefCounted>) -> Self {
        // let gd = base.to_init_gd();

        eprintln!("---- before to_init_gd() ----");
        base.to_init_gd(); // Immediately dropped.
        eprintln!("---- after to_init_gd() ----");

        // let _local_copy = base.to_init_gd(); // At end of scope.
        // let moved_out = Some(base.to_init_gd()); // Moved out.
        // std::mem::forget(moved_out);

        // drop(gd);

        // let refc: &mut Gd<RefCounted> = base.as_init_gd();
        // let refc = refc.get_reference_count();
        // println!("Inside init(): refc={}", refc);
        Self { base }
    }
}

impl RefcBased {
    fn with_split() -> (Gd<Self>, Gd<RefCounted>) {
        let mut moved_out = None;

        let self_gd = Gd::from_init_fn(|mut base| {
            let gd = base.to_init_gd();

            base.to_init_gd(); // Immediately dropped.

            let _local_copy = base.to_init_gd(); // At end of scope.
            moved_out = Some(base.to_init_gd()); // Moved out.

            drop(gd);

            let refc: &mut Gd<RefCounted> = base.as_init_gd();
            let refc = refc.get_reference_count();
            println!("Inside init(): refc={}", refc);
            Self { base }
        });

        (self_gd, moved_out.unwrap())
    }
}
