/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Test that all important dyn-related symbols are in the prelude.
use godot::prelude::*;

use crate::framework::{expect_panic, itest};

#[itest]
fn dyn_gd_creation_bind() {
    // Type inference on this works because only 1 AsDyn<...> trait is implemented for RefcHealth. It would fail with another.
    let _unused = Gd::from_object(RefcHealth { hp: 1 }).into_dyn();

    let user_obj = RefcHealth { hp: 34 };
    let mut dyn_gd = Gd::from_object(user_obj).into_dyn();

    {
        // Exclusive bind.
        // Interesting: this can be type inferred because RefcHealth implements only 1 AsDyn<...> trait.
        // If there were another, type inference would fail.
        let mut health = dyn_gd.dyn_bind_mut();
        health.deal_damage(4);
    }
    {
        // Test multiple shared binds.
        let health_a = dyn_gd.dyn_bind();
        let health_b = dyn_gd.dyn_bind();

        assert_eq!(health_b.get_hitpoints(), 30);
        assert_eq!(health_a.get_hitpoints(), 30);
    }
    {
        let mut health = dyn_gd.dyn_bind_mut();
        health.kill();

        assert_eq!(health.get_hitpoints(), 0);
    }
}

#[itest]
fn dyn_gd_creation_deref() {
    let obj = Gd::from_object(RefcHealth { hp: 100 });
    let original_id = obj.instance_id();

    // Type can be safely inferred.
    let mut obj = obj.into_dyn();

    let dyn_id = obj.instance_id();
    assert_eq!(dyn_id, original_id);

    deal_20_damage(&mut *obj.dyn_bind_mut());
    assert_eq!(obj.dyn_bind().get_hitpoints(), 80);
}

#[itest]
fn dyn_gd_creation_deref_multiple_traits() {
    let original_obj = foreign::NodeHealth::new_alloc();
    let original_id = original_obj.instance_id();

    // Type can be inferred because `Health` explicitly declares a 'static bound.
    let mut obj = original_obj.clone().into_dyn();

    let dyn_id = obj.instance_id();
    assert_eq!(dyn_id, original_id);

    deal_20_damage(&mut *obj.dyn_bind_mut());
    assert_eq!(obj.dyn_bind().get_hitpoints(), 80);

    // Otherwise type inference doesn't work and type must be explicitly declared.
    let mut obj = original_obj
        .clone()
        .into_dyn::<dyn InstanceIdProvider<Id = InstanceId>>();
    assert_eq!(get_instance_id(&mut *obj.dyn_bind_mut()), original_id);

    // Not recommended – for presentational purposes only.
    // Works because 'static bound on type is enforced in function signature.
    // I.e. this wouldn't work with fn get_instance_id(...).
    let mut obj = original_obj.into_dyn();
    get_instance_id_explicit_static_bound(&mut *obj.dyn_bind_mut());

    obj.free();
}

fn deal_20_damage(h: &mut dyn Health) {
    h.deal_damage(20);
}

fn get_instance_id(i: &mut dyn InstanceIdProvider<Id = InstanceId>) -> InstanceId {
    i.get_id_dynamic()
}

fn get_instance_id_explicit_static_bound(
    i: &mut (dyn InstanceIdProvider<Id = InstanceId> + 'static),
) -> InstanceId {
    i.get_id_dynamic()
}

#[itest]
fn dyn_gd_upcast() {
    let original = foreign::NodeHealth::new_alloc();
    let original_copy = original.clone();

    let concrete = original.into_dyn::<dyn Health>();

    let mut node = concrete.clone().upcast::<Node>();
    let object = concrete.upcast::<Object>();

    node.dyn_bind_mut().deal_damage(25);

    // Make sure identity is intact.
    assert_eq!(node.instance_id(), original_copy.instance_id());

    // Ensure applied to the object polymorphically. Concrete object can access bind(), no dyn_bind().
    assert_eq!(original_copy.bind().get_hitpoints(), 75);

    // Check also another angle (via Object). Here dyn_bind().
    assert_eq!(object.dyn_bind().get_hitpoints(), 75);

    node.free();
}

#[itest]
fn dyn_gd_downcast() {
    let original = Gd::from_object(RefcHealth { hp: 20 }).into_dyn();
    let mut object = original.upcast::<Object>();

    object.dyn_bind_mut().deal_damage(7);

    let failed = object.try_cast::<foreign::NodeHealth>();
    let object = failed.expect_err("DynGd::try_cast() succeeded, but should have failed");

    let refc = object.cast::<RefCounted>();
    assert_eq!(refc.dyn_bind().get_hitpoints(), 13);

    let back = refc
        .try_cast::<RefcHealth>()
        .expect("DynGd::try_cast() should have succeeded");
    assert_eq!(back.bind().get_hitpoints(), 13);
}

#[itest]
fn dyn_gd_debug() {
    let node = foreign::NodeHealth::new_alloc();
    let id = node.instance_id();

    let node = node.into_dyn::<dyn Health>();

    let actual = format!(".:{node:?}:.");
    let expected = format!(".:DynGd {{ id: {id}, class: NodeHealth, trait: dyn Health }}:.");

    assert_eq!(actual, expected);

    let node = node
        .into_gd()
        .into_dyn::<dyn InstanceIdProvider<Id = InstanceId>>();
    let actual = format!(".:{node:?}:.");
    let expected = format!(".:DynGd {{ id: {id}, class: NodeHealth, trait: dyn InstanceIdProvider<Id = InstanceId> }}:.");

    assert_eq!(actual, expected);

    node.free();
}

#[itest]
fn dyn_gd_display() {
    let obj = Gd::from_object(RefcHealth { hp: 55 }).into_dyn();

    let actual = format!("{obj}");
    let expected = "RefcHealth(hp=55)";

    assert_eq!(actual, expected);
}

#[itest]
fn dyn_gd_eq() {
    let gd = Gd::from_object(RefcHealth { hp: 55 });
    let a = gd.clone().into_dyn();
    let b = gd.into_dyn();
    let c = b.clone();

    assert_eq!(a, b);
    assert_eq!(a, c);
    assert_eq!(b, c);

    let x = Gd::from_object(RefcHealth { hp: 55 }).into_dyn();

    assert_ne!(a, x);
}

#[itest]
fn dyn_gd_hash() {
    use godot::sys::hash_value;

    let gd = Gd::from_object(RefcHealth { hp: 55 });
    let a = gd.clone().into_dyn();
    let b = gd.into_dyn();
    let c = b.clone();

    assert_eq!(hash_value(&a), hash_value(&b));
    assert_eq!(hash_value(&a), hash_value(&c));
    assert_eq!(hash_value(&b), hash_value(&c));

    let x = Gd::from_object(RefcHealth { hp: 55 }).into_dyn();

    // Not guaranteed, but exceedingly likely.
    assert_ne!(hash_value(&a), hash_value(&x));
}

#[itest]
fn dyn_gd_exclusive_guard() {
    let mut a = foreign::NodeHealth::new_alloc().into_dyn::<dyn Health>();
    let mut b = a.clone();

    let guard = a.dyn_bind_mut();

    expect_panic(
        "Cannot acquire dyn_bind() guard while dyn_bind_mut() is held",
        || {
            let _ = b.dyn_bind();
        },
    );
    expect_panic(
        "Cannot acquire 2nd dyn_bind_mut() guard while dyn_bind_mut() is held",
        || {
            let _ = b.dyn_bind_mut();
        },
    );
    expect_panic("Cannot free object while dyn_bind_mut() is held", || {
        b.free();
    });

    drop(guard);
    a.free(); // now allowed.
}

#[itest]
fn dyn_gd_shared_guard() {
    let a = foreign::NodeHealth::new_alloc().into_dyn::<dyn Health>();
    let b = a.clone();
    let mut c = a.clone();

    let guard_a = a.dyn_bind();

    // CAN acquire another dyn_bind() while an existing one exists.
    let guard_b = b.dyn_bind();
    drop(guard_a);

    // guard_b still alive here.
    expect_panic(
        "Cannot acquire dyn_bind_mut() guard while dyn_bind() is held",
        || {
            let _ = c.dyn_bind_mut();
        },
    );

    // guard_b still alive here.
    expect_panic("Cannot free object while dyn_bind() is held", || {
        c.free();
    });

    drop(guard_b);
    a.free(); // now allowed.
}

#[itest]
fn dyn_gd_downgrade() {
    let dyn_gd = RefcHealth::new_gd().into_dyn();
    let dyn_id = dyn_gd.instance_id();

    let gd = dyn_gd.into_gd();

    assert_eq!(gd.bind().get_hitpoints(), 0); // default hp is 0.
    assert_eq!(gd.instance_id(), dyn_id);
}

#[itest]
fn dyn_gd_call_godot_method() {
    let mut node = foreign::NodeHealth::new_alloc().into_dyn::<dyn Health>();

    node.set_name("dyn-name!");
    assert_eq!(node.get_name(), "dyn-name!".into());

    node.free();
}

#[itest]
fn dyn_gd_pass_to_godot_api() {
    let child = foreign::NodeHealth::new_alloc().into_dyn::<dyn Health>();

    let mut parent = Node::new_alloc();
    parent.add_child(&child);

    assert_eq!(child.get_parent().as_ref(), Some(&parent));

    parent.free();
}

#[itest]
fn dyn_gd_variant_conversions() {
    let node = foreign::NodeHealth::new_alloc();
    let original_id = node.instance_id();

    let variant = node.to_variant();

    // Convert to different levels of DynGd:

    let back: DynGd<foreign::NodeHealth, dyn Health> = variant.to();
    assert_eq!(back.bind().get_hitpoints(), 100);
    assert_eq!(back.instance_id(), original_id);

    let back: DynGd<Node, dyn Health> = variant.to();
    assert_eq!(back.dyn_bind().get_hitpoints(), 100);
    assert_eq!(back.instance_id(), original_id);

    let back: DynGd<Object, dyn Health> = variant.to();
    assert_eq!(back.dyn_bind().get_hitpoints(), 100);
    assert_eq!(back.instance_id(), original_id);

    // Convert to different DynGd:

    let back: DynGd<foreign::NodeHealth, dyn InstanceIdProvider<Id = InstanceId>> = variant.to();
    assert_eq!(back.dyn_bind().get_id_dynamic(), original_id);

    let back: DynGd<Node, dyn InstanceIdProvider<Id = InstanceId>> = variant.to();
    assert_eq!(back.dyn_bind().get_id_dynamic(), original_id);

    let back: DynGd<Object, dyn InstanceIdProvider<Id = InstanceId>> = variant.to();
    assert_eq!(back.dyn_bind().get_id_dynamic(), original_id);

    // Convert to different levels of Gd:

    let back: Gd<foreign::NodeHealth> = variant.to();
    assert_eq!(back.bind().get_hitpoints(), 100);
    assert_eq!(back.instance_id(), original_id);

    let back: Gd<Object> = variant.to();
    assert_eq!(back.instance_id(), original_id);

    node.free();
}

#[itest]
fn dyn_gd_object_conversions() {
    let node = foreign::NodeHealth::new_alloc().upcast::<Node>();
    let original_id = node.instance_id();

    // Convert to different levels of DynGd:
    let back: DynGd<Node, dyn Health> = node
        .try_dynify()
        .expect("Gd::try_dynify() should succeed.")
        .cast();
    assert_eq!(back.dyn_bind().get_hitpoints(), 100);
    assert_eq!(back.instance_id(), original_id);

    let obj = back.into_gd().upcast::<Object>();
    let back: DynGd<Object, dyn Health> =
        obj.try_dynify().expect("Gd::try_dynify() should succeed.");
    assert_eq!(back.dyn_bind().get_hitpoints(), 100);
    assert_eq!(back.instance_id(), original_id);

    // Back to NodeHealth.
    let node = back.cast::<foreign::NodeHealth>();
    assert_eq!(node.bind().get_hitpoints(), 100);
    assert_eq!(node.instance_id(), original_id);

    // Convert to different DynGd.
    let obj = node.into_gd().upcast::<Node>();
    let back: DynGd<Node, dyn InstanceIdProvider<Id = InstanceId>> =
        obj.try_dynify().expect("Gd::try_dynify() should succeed.");
    assert_eq!(back.dyn_bind().get_id_dynamic(), original_id);

    let obj = back.into_gd().upcast::<Object>();
    let back: DynGd<Object, dyn InstanceIdProvider<Id = InstanceId>> =
        obj.try_dynify().expect("Gd::try_dynify() should succeed.");
    assert_eq!(back.dyn_bind().get_id_dynamic(), original_id);

    back.free()
}

#[itest]
fn dyn_gd_object_conversion_failures() {
    // Unregistered trait conversion failure.
    trait UnrelatedTrait {}

    let node = foreign::NodeHealth::new_alloc().upcast::<Node>();
    let original_id = node.instance_id();
    let back = node.try_dynify::<dyn UnrelatedTrait>();
    let node = back.expect_err("Gd::try_dynify() should have failed");

    // `Gd::try_dynify()` should return the original instance on failure, similarly to `Gd::try_cast()`.
    assert_eq!(original_id, node.instance_id());

    // Unimplemented trait conversion failures.
    let back = node.try_dynify::<dyn InstanceIdProvider<Id = i32>>();
    let node = back.expect_err("Gd::try_dynify() should have failed");
    assert_eq!(original_id, node.instance_id());

    let obj = RefCounted::new_gd();
    let original_id = obj.instance_id();
    let back = obj.try_dynify::<dyn Health>();
    let obj = back.expect_err("Gd::try_dynify() should have failed");
    assert_eq!(original_id, obj.instance_id());

    node.free();
}

#[itest]
fn dyn_gd_store_in_godot_array() {
    let a = Gd::from_object(RefcHealth { hp: 33 }).into_dyn();
    let b = foreign::NodeHealth::new_alloc().into_dyn();

    // Also tests AsArg impl for DynGd, which previously suffered from UB.
    let array: Array<DynGd<Object, _>> = array![&a, &b];

    assert_eq!(array.at(0).dyn_bind().get_hitpoints(), 33);
    assert_eq!(array.at(1).dyn_bind().get_hitpoints(), 100);

    array.at(1).free();

    // Used to support type inference of array![]. Not anymore with unified AsArg<..> + upcast support.
    /*
    let c: DynGd<RefcHealth, dyn Health> = Gd::from_object(RefcHealth { hp: 33 }).into_dyn();
    let c = c.upcast::<RefCounted>();
    let array_inferred /*: Array<DynGd<RefCounted, _>>*/ = array![&c];
    assert_eq!(array_inferred.at(0).dyn_bind().get_hitpoints(), 33);
    */
}

#[itest]
fn dyn_gd_error_unregistered_trait() {
    trait UnrelatedTrait {}
    let node = foreign::NodeHealth::new_alloc().into_dyn::<dyn Health>();

    let variant = node.to_variant();

    let back = variant.try_to::<DynGd<foreign::NodeHealth, dyn UnrelatedTrait>>();

    // The conversion fails before a DynGd is created, so Display still operates on the Gd.
    let node = node.into_gd();

    let err = back.expect_err("DynGd::try_to() should have failed");
    let expected_err = // Variant Debug uses "VariantGd" prefix.
        format!("trait `dyn UnrelatedTrait` has not been registered with #[godot_dyn]: Variant{node:?}");

    assert_eq!(err.to_string(), expected_err);

    let back = variant.try_to::<DynGd<foreign::NodeHealth, dyn InstanceIdProvider<Id = i32>>>();

    // Variant Debug uses "VariantGd" prefix.
    let err = back.expect_err("DynGd::try_to() should have failed");
    let expected_err = format!("trait `dyn InstanceIdProvider<Id = i32>` has not been registered with #[godot_dyn]: Variant{node:?}");

    assert_eq!(err.to_string(), expected_err);

    node.free();
}

#[itest]
fn dyn_gd_error_unimplemented_trait() {
    let obj = RefCounted::new_gd();

    let variant = obj.to_variant();
    let back = variant.try_to::<DynGd<RefCounted, dyn Health>>();

    let err = back.expect_err("DynGd::try_to() should have failed");

    let refc_id = obj.instance_id().to_i64();
    let expected_debug = format!(
        "none of the classes derived from `RefCounted` have been linked to trait `dyn Health` with #[godot_dyn]: \
         VariantGd {{ id: {refc_id}, class: RefCounted, refc: 3 }}"
    );
    assert_eq!(err.to_string(), expected_debug);

    let node = foreign::NodeHealth::new_alloc();
    let variant = node.to_variant();
    let back = variant.try_to::<DynGd<foreign::NodeHealth, dyn InstanceIdProvider<Id = f32>>>();

    let err = back.expect_err("DynGd::try_to() should have failed");

    // NodeHealth is manually managed (inherits Node), so no refcount in debug output.
    let node_id = node.instance_id().to_i64();
    let expected_debug = format!(
        "none of the classes derived from `NodeHealth` have been linked to trait `dyn InstanceIdProvider<Id = f32>` with #[godot_dyn]: \
         VariantGd {{ id: {node_id}, class: NodeHealth }}"
    );
    assert_eq!(err.to_string(), expected_debug);

    node.free();
}

#[itest]
fn dyn_gd_free_while_dyn_bound() {
    let mut obj = foreign::NodeHealth::new_alloc().into_dyn::<dyn Health>();

    {
        let copy = obj.clone();
        let _guard = obj.dyn_bind();

        expect_panic("Cannot free while dyn_bind() guard is held", || {
            copy.free();
        });
    }
    {
        let copy = obj.clone();
        let _guard = obj.dyn_bind_mut();

        expect_panic("Cannot free while dyn_bind_mut() guard is held", || {
            copy.free();
        });
    }

    // Now allowed.
    obj.free();
}

#[itest]
fn dyn_gd_multiple_traits() {
    let obj = foreign::NodeHealth::new_alloc();
    let original_id = obj.instance_id();

    let obj = obj
        .into_dyn::<dyn InstanceIdProvider<Id = InstanceId>>()
        .upcast::<Node>();
    let id = obj.dyn_bind().get_id_dynamic();
    assert_eq!(id, original_id);

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Example symbols

// 'static bound must be explicitly declared to make type inference work.
trait Health: 'static {
    fn get_hitpoints(&self) -> u8;

    fn deal_damage(&mut self, damage: u8);

    fn kill(&mut self) {
        self.deal_damage(self.get_hitpoints());
    }
}

#[derive(GodotClass)]
#[class(init)]
struct RefcHealth {
    hp: u8,
}

#[godot_api]
impl IRefCounted for RefcHealth {
    fn to_string(&self) -> GString {
        format!("RefcHealth(hp={})", self.hp).into()
    }
}

// Pretend NodeHealth is defined somewhere else, with a default constructor but
// no knowledge of health. We retrofit the property via Godot "meta" key-values.
mod foreign {
    use super::*;

    #[derive(GodotClass)]
    #[class(init, base=Node)]
    pub struct NodeHealth {
        base: Base<Node>,
    }
}

#[godot_dyn]
impl Health for RefcHealth {
    fn get_hitpoints(&self) -> u8 {
        self.hp
    }

    fn deal_damage(&mut self, damage: u8) {
        self.hp -= damage;
    }
}

#[godot_dyn]
impl Health for foreign::NodeHealth {
    fn get_hitpoints(&self) -> u8 {
        if self.base().has_meta("hp") {
            self.base().get_meta("hp").to::<u8>()
        } else {
            100 // initial value, if nothing set.
        }
    }

    fn deal_damage(&mut self, damage: u8) {
        let new_hp = self.get_hitpoints() - damage;
        self.base_mut().set_meta("hp", &new_hp.to_variant());
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Check that one class can implement two or more traits.

trait InstanceIdProvider {
    type Id;
    fn get_id_dynamic(&self) -> Self::Id;
}

#[godot_dyn]
impl InstanceIdProvider for foreign::NodeHealth {
    type Id = InstanceId;
    fn get_id_dynamic(&self) -> Self::Id {
        self.base().instance_id()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Checks if DynGd can be properly used as a `#[var]`.
// All classes can be used as a `#[var]` for `DynGd<T, D>`.

#[derive(GodotClass)]
#[class(init)]
struct RefcDynGdVarDeclarer {
    #[var]
    first: Option<DynGd<Object, dyn Health>>,
    #[var]
    second: Option<DynGd<foreign::NodeHealth, dyn InstanceIdProvider<Id = InstanceId>>>,
}

// Implementation created only to register the DynGd `HealthWithAssociatedType<HealthType=f32>` trait.
// Pointless trait, but tests proper conversion.
#[godot_dyn]
impl InstanceIdProvider for RefcDynGdVarDeclarer {
    type Id = f32;
    fn get_id_dynamic(&self) -> Self::Id {
        42.0
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Checks if `#[export]`s for DynGd can be properly auto-generated.
// Only built-in classes can be used as an `#[export]` for `DynGd<T, D>`.

#[derive(GodotClass)]
#[class(init, base=Node)]
struct DynGdExporter {
    #[export]
    first: Option<DynGd<Resource, dyn Health>>,
    #[export]
    second: OnEditor<DynGd<Resource, dyn Health>>,
}
