/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{VarDictionary, Variant};
use godot::classes::class_macros::private::virtuals::Os::Vector3;
use godot::classes::mesh::PrimitiveType;
use godot::classes::{ArrayMesh, Node, Resource};
use godot::meta::owned_into_arg;
use godot::obj::{Gd, NewAlloc, NewGd, Unique};

use crate::framework::{expect_panic_or_nothing, itest};

#[itest]
fn valid_use_case() {
    std::thread::spawn(|| {
        let shadow_mesh: Unique<Gd<ArrayMesh>> = Unique::new_gd();
        let mut mesh: Unique<Gd<ArrayMesh>> = Unique::new_gd();

        mesh.apply_gd(|mesh| {
            mesh.set_shadow_mesh(shadow_mesh);
        });
    })
    .join()
    .unwrap();
}

#[itest]
fn trying_to_cheat() {
    std::thread::spawn(|| {
        thread_local! {
            static CHILD_NODE: Gd<Node> = Node::new_alloc();
        }

        let mut node = Unique::<Gd<Node>>::new_alloc();
        let mut dict = Unique::<VarDictionary>::new();

        expect_panic_or_nothing("thread should panic when passing by ref", || {
            node.apply_gd(|node: &mut Node| {
                let owned_node = CHILD_NODE.with(|value: &Gd<Node>| value.clone());
                node.add_child(&owned_node);
            });
        });

        expect_panic_or_nothing("thread should panic when passing by option ref", || {
            node.apply_gd(|node: &mut Node| {
                let owned_node = CHILD_NODE.with(|value: &Gd<Node>| value.clone());
                node.set_owner(&owned_node);
            });
        });

        expect_panic_or_nothing("thread should panic when passing by variant", || {
            dict.apply(|dict| {
                dict.contains_key(&Variant::nil());
            });
        });

        expect_panic_or_nothing("thread should panic when passing by own reference", || {
            dict.apply(|dict| {
                let _ = dict.insert("key", &dict.clone());
            });
        });

        expect_panic_or_nothing("thread should panic when passing by Gd<T> by value", || {
            node.apply_gd(|node: &mut Node| {
                let owned_node = CHILD_NODE.with(|value: &Gd<Node>| value.clone());
                node.add_child(owned_into_arg(owned_node));
            });
        });

        // Test clean-up to avoid memory leaks.
        CHILD_NODE.with(|value| value.clone().free());
        node.share().free();
    })
    .join()
    .unwrap();
}

#[itest]
fn recursive_unique_check() {
    let resource = Resource::new_gd();

    let unique_res = Unique::try_from_ref_counted(resource);

    assert!(
        unique_res.is_some(),
        "Uniqueness verification with a single ref should succeed"
    );

    let resource = unique_res.unwrap().share();
    let second_ref = resource.clone();

    assert!(
        Unique::try_from_ref_counted(resource).is_none(),
        "Uniqueness verification with two refs should fail"
    );

    drop(second_ref);
}

#[itest]
#[cfg(feature = "codegen-full")]
fn sub_thread_surface_tool() {
    use godot::classes::SurfaceTool;

    let result = std::thread::spawn(|| {
        let mut builder: Gd<SurfaceTool> = SurfaceTool::new_gd();

        builder.begin(PrimitiveType::TRIANGLES);
        builder.add_vertex(Vector3::new(1.0, 1.0, 1.0));
        builder.add_vertex(Vector3::new(1.0, 2.0, 1.0));
        builder.add_vertex(Vector3::new(1.0, 1.0, 2.0));

        let existing_mesh: Unique<Gd<ArrayMesh>> = Unique::new_gd();

        let result = builder.commit_ex().existing(existing_mesh).done().unwrap();

        Unique::try_from_ref_counted(result).unwrap()
    })
    .join()
    .expect("sub-thread should not have paniced");

    assert_eq!(result.share().get_reference_count(), 1);
}
