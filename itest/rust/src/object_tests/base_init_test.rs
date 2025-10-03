/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::real_consts::FRAC_PI_3;
use godot::builtin::{Array, PackedArray, Variant, Vector2};
use godot::classes::notify::ObjectNotification;
use godot::classes::{mesh, ArrayMesh, ClassDb, IArrayMesh, IRefCounted, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, IndexEnum, InstanceId, NewAlloc, NewGd, Singleton, WithBaseField};
use godot::register::{godot_api, GodotClass};
use godot::task::TaskHandle;

use crate::framework::{expect_panic, itest, next_frame};
use crate::object_tests::base_test::{Based, RefcBased};

#[itest]
fn base_init_propagation() {
    // ::<Based> necessary here, for type inference of to_init_gd() return type.
    let obj = Gd::<Based>::from_init_fn(|base| {
        // Test both temporary + local-variable syntax.
        base.to_init_gd().set_rotation(FRAC_PI_3);

        let mut gd = base.to_init_gd();
        gd.set_position(Vector2::new(100.0, 200.0));

        Based { base, i: 456 }
    });

    // Check that values are propagated to derived object.
    let guard = obj.bind();
    assert_eq!(guard.i, 456);
    assert_eq!(guard.base().get_rotation(), FRAC_PI_3);
    assert_eq!(guard.base().get_position(), Vector2::new(100.0, 200.0));
    drop(guard);

    obj.free();
}

// This isn't recommended, but test what happens if someone clones and stores the Gd<T>.
#[itest]
fn base_init_extracted_gd() {
    let mut extractor = None;

    let obj = Gd::<Based>::from_init_fn(|base| {
        extractor = Some(base.to_init_gd());

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
fn base_init_freed_gd() {
    let mut free_executed = false;

    expect_panic("base object is destroyed", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let obj = base.to_init_gd();
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

// Verifies that there are no panics, memory leaks or UB in basic case.
#[itest]
fn base_init_refcounted_simple() {
    let _obj = Gd::from_init_fn(|base| {
        drop(base.to_init_gd());

        RefcBased { base }
    });
}

// Tests that the auto-decrement of surplus references also works when instantiated through the engine.
#[itest(async)]
fn base_init_refcounted_from_engine() -> TaskHandle {
    let db = ClassDb::singleton();
    let obj = db.instantiate("RefcBased").to::<Gd<RefcBased>>();

    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

#[itest(async)]
fn base_init_refcounted_from_rust() -> TaskHandle {
    let obj = RefcBased::new_gd();

    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

#[itest(async)]
fn base_init_refcounted_complex() -> TaskHandle {
    // Instantiate with multiple Gd<T> references.
    let id_simple = verify_complex_init(RefcBased::split_simple());
    let id_intermixed = verify_complex_init(RefcBased::split_intermixed());

    next_frame(move || {
        assert!(!id_simple.lookup_validity(), "object destroyed eventually");
        assert!(
            !id_intermixed.lookup_validity(),
            "object destroyed eventually"
        );
    })
}

fn verify_complex_init((obj, base): (Gd<RefcBased>, Gd<RefCounted>)) -> InstanceId {
    let id = obj.instance_id();

    assert_eq!(obj.instance_id(), base.instance_id());
    assert_eq!(base.get_reference_count(), 3);
    assert_eq!(obj.get_reference_count(), 3);

    drop(base);
    assert_eq!(obj.get_reference_count(), 2);
    assert_eq!(obj.get_reference_count(), 2);
    drop(obj);

    // Not dead yet.
    assert!(id.lookup_validity(), "object retained (dec-ref deferred)");
    id
}

#[cfg(debug_assertions)]
#[itest]
fn base_init_outside_init() {
    let mut obj = Based::new_alloc();

    expect_panic("to_init_gd() outside init() function", || {
        let guard = obj.bind_mut();
        let _gd = guard.base.to_init_gd(); // Panics in Debug builds.
    });

    obj.free();
}

#[cfg(debug_assertions)]
#[itest]
fn base_init_to_gd() {
    expect_panic("WithBaseField::to_gd() inside init() function", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let temp_obj = Based { base, i: 999 };

            // Call to self.to_gd() during initialization should panic in Debug builds.
            let _gd = godot::obj::WithBaseField::to_gd(&temp_obj);

            temp_obj
        });
    });
}

#[derive(GodotClass)]
#[class(init)]
struct RefcPostinit {
    pub base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for RefcPostinit {
    fn on_notification(&mut self, what: ObjectNotification) {
        if what == ObjectNotification::POSTINITIALIZE {
            self.base
                .to_init_gd()
                .set_meta("meta", &"postinited".to_variant());
        }
    }
}

#[cfg(since_api = "4.4")]
#[itest(async)]
fn base_postinit_refcounted() -> TaskHandle {
    let obj = RefcPostinit::new_gd();
    assert_eq!(obj.get_meta("meta"), "postinited".to_variant());
    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

fn make_mesh_arrays() -> Array<Variant> {
    let mut arrays = Array::new();
    arrays.resize(mesh::ArrayType::ENUMERATOR_COUNT, &Variant::nil());
    let indices = PackedArray::<i32>::from([0, 1, 2]);
    let vertex = PackedArray::<Vector2>::from([
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(0.0, 1.0),
    ]);
    arrays.set(mesh::ArrayType::INDEX.to_index(), &indices.to_variant());
    arrays.set(mesh::ArrayType::VERTEX.to_index(), &vertex.to_variant());
    arrays
}

#[derive(GodotClass)]
#[class(base=ArrayMesh)]
struct InitArrayMeshTest {
    base: Base<ArrayMesh>,
}

#[rustfmt::skip]
#[godot_api]
impl IArrayMesh for InitArrayMeshTest {
    fn init(base: Base<ArrayMesh>) -> Self {
        let mut sf = Self { base };
        sf.base_mut()
            .add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &make_mesh_arrays());
        sf
    }

    fn on_notification(&mut self, what: ObjectNotification) {
        if what == ObjectNotification::PREDELETE {
            let arr = make_mesh_arrays();
            self.base_mut().add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arr);

            assert_eq!(self.base().get_surface_count(), 2);
            assert_eq!(self.base().surface_get_arrays(0), arr);
            assert_eq!(self.base().surface_get_arrays(1), arr);
        }
    }

    fn get_surface_count(&self) -> i32 { unreachable!(); }
    fn surface_get_array_len(&self, _index: i32) -> i32 { unreachable!(); }
    fn surface_get_array_index_len(&self, _index: i32) -> i32 { unreachable!(); }
    fn surface_get_arrays(&self, _index: i32) -> Array<Variant> { unreachable!(); }
    fn surface_get_blend_shape_arrays(&self, _index: i32) -> Array<Array<Variant>> { unreachable!(); }
    fn surface_get_lods(&self, _index: i32) -> godot::builtin::Dictionary { unreachable!(); }
    fn surface_get_format(&self, _index: i32) -> u32 { unreachable!(); }
    fn surface_get_primitive_type(&self, _index: i32) -> u32 { unreachable!(); }
    #[cfg(feature = "codegen-full")]
    fn surface_set_material(&mut self, _index: i32, _material: Option<Gd<godot::classes::Material>>) { unreachable!(); }
    #[cfg(feature = "codegen-full")]
    fn surface_get_material(&self, _index: i32) -> Option<Gd<godot::classes::Material>> { unreachable!(); }
    fn get_blend_shape_count(&self) -> i32 { unreachable!(); }
    fn get_blend_shape_name(&self, _index: i32) -> godot::builtin::StringName { unreachable!(); }
    fn set_blend_shape_name(&mut self, _index: i32, _name: godot::builtin::StringName){ unreachable!(); }
    fn get_aabb(&self) -> godot::builtin::Aabb { unreachable!(); }
}

#[itest]
fn base_mut_init_array_mesh() {
    let mesh = InitArrayMeshTest::new_gd();
    assert_eq!(mesh.get_surface_count(), 1);
    assert_eq!(mesh.surface_get_arrays(0), make_mesh_arrays());
}
