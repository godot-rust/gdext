/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{vslice, Variant};
use godot::classes::{ClassDb, Node, RefCounted, ResourceFormatLoader, ResourceLoader};
use godot::global;
use godot::meta::ToGodot;
use godot::obj::{Gd, NewAlloc, NewGd, Singleton};
use godot::register::{godot_api, GodotClass};

use crate::framework::{create_gdscript, itest};
use crate::object_tests::object_test::{user_refc_instance, RefcPayload};

/*
#[itest]
fn object_arg_owned() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(manual, "name", &Variant::from("hello"));
        let b = db.class_set_property(refc, "value", &Variant::from(-123));
        (a, b)
    });
}
*/

#[itest]
fn object_arg_borrowed() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(&manual, "name", &Variant::from("hello"));
        let b = db.class_set_property(&refc, "value", &Variant::from(-123));
        (a, b)
    });
}

#[itest]
fn object_arg_borrowed_mut() {
    with_objects(|mut manual, mut refc| {
        let db = ClassDb::singleton();

        let manual_ref = &mut manual;
        let refc_ref = &mut refc;

        let a = db.class_set_property(&*manual_ref, "name", &Variant::from("hello"));
        let b = db.class_set_property(&*refc_ref, "value", &Variant::from(-123));
        (a, b)
    });
}

/*
#[itest]
fn object_arg_option_owned() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(Some(manual), "name", &Variant::from("hello"));
        let b = db.class_set_property(Some(refc), "value", &Variant::from(-123));
        (a, b)
    });
}
*/

#[itest]
fn object_arg_option_borrowed() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(Some(&manual), "name", &Variant::from("hello"));
        let b = db.class_set_property(Some(&refc), "value", &Variant::from(-123));
        (a, b)
    });
}

/*
#[itest]
fn object_arg_option_borrowed_outer() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(&Some(manual), "name", &Variant::from("hello"));
        let b = db.class_set_property(&Some(refc), "value", &Variant::from(-123));
        (a, b)
    });
}
*/

#[itest]
fn object_arg_option_borrowed_mut() {
    // If you have an Option<&mut Gd<T>>, you can use as_deref() to get Option<&Gd<T>>.

    with_objects(|mut manual, mut refc| {
        let db = ClassDb::singleton();

        let manual_opt: Option<&mut Gd<Node>> = Some(&mut manual);
        let refc_opt: Option<&mut Gd<RefcPayload>> = Some(&mut refc);

        let a = db.class_set_property(manual_opt.as_deref(), "name", &Variant::from("hello"));
        let b = db.class_set_property(refc_opt.as_deref(), "value", &Variant::from(-123));
        (a, b)
    });
}

#[itest]
fn object_arg_option_none() {
    let manual: Option<Gd<Node>> = None;
    let refc: Option<Gd<RefcPayload>> = None;

    // Will emit errors but should not crash.
    let db = ClassDb::singleton();
    let error = db.class_set_property(manual.as_ref(), "name", &Variant::from("hello"));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);

    let error = db.class_set_property(refc.as_ref(), "value", &Variant::from(-123));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);
}

#[itest]
fn object_arg_null_arg() {
    // Will emit errors but should not crash.
    let db = ClassDb::singleton();
    let error = db.class_set_property(Gd::null_arg(), "name", &Variant::from("hello"));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);

    let error = db.class_set_property(Gd::null_arg(), "value", &Variant::from(-123));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);
}

// Regression test for https://github.com/godot-rust/gdext/issues/835.
#[itest]
fn object_arg_owned_default_params() {
    // Calls the _ex() variant behind the scenes.
    let a = ResourceFormatLoader::new_gd();
    let b = ResourceFormatLoader::new_gd();

    // Use direct and explicit _ex() call syntax.
    ResourceLoader::singleton().add_resource_format_loader(&a);
    ResourceLoader::singleton()
        .add_resource_format_loader_ex(&b)
        .done();

    // Clean up (no leaks).
    ResourceLoader::singleton().remove_resource_format_loader(&a);
    ResourceLoader::singleton().remove_resource_format_loader(&b);
}

// Gd<RefCounted> passed to GDScript should not create unnecessary clones.
#[itest]
fn refcount_asarg_gdscript_calls() {
    let script = create_gdscript(
        r#"
extends RefCounted

func observe_refcount_ptrcall(obj: RefCounted) -> int:
    return obj.get_reference_count()

func observe_refcount_varcall(obj) -> int:
    if obj == null:
        return 0
    return obj.get_reference_count()
"#,
    );

    let mut test_instance = RefCounted::new_gd();
    test_instance.set_script(&script);

    // Already pack into Variant, to have 1 less reference count increment.
    let refc = RefCounted::new_gd().to_variant();
    assert_eq!(refc.call("get_reference_count", &[]), 1.to_variant());

    let refcount_typed: i32 = test_instance
        .call("observe_refcount_ptrcall", &[refc])
        .to::<i32>();

    let refc = RefCounted::new_gd().to_variant();
    let refcount_untyped = test_instance
        .call("observe_refcount_varcall", &[refc])
        .to::<i32>();

    let refcount_none = test_instance
        .call("observe_refcount_varcall", vslice![Variant::nil()])
        .to::<i32>();

    // Both should result in refcount 2: 1 variant in Rust + 1 reference created on GDScript side.
    assert_eq!(refcount_typed, 2, "typed GDScript param (ptrcall)");
    assert_eq!(refcount_untyped, 2, "untyped GDScript param (varcall)");
    assert_eq!(refcount_none, 0, "None/null parameter should return 0");
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests with engine APIs + AsArg below, in module.

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
pub struct RefCountAsArgTest;

#[godot_api]
impl RefCountAsArgTest {
    #[func]
    fn accept_option_refcounted(&self, obj: Option<Gd<RefCounted>>) -> i32 {
        match obj {
            Some(gd) => gd.get_reference_count(),
            None => 0,
        }
    }

    #[func]
    fn accept_object(&self, obj: Gd<godot::classes::Object>) -> bool {
        // Just verify we can receive an Object (for upcast testing).
        !obj.get_class().is_empty()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers

fn with_objects<F>(f: F)
where
    F: FnOnce(Gd<Node>, Gd<RefcPayload>) -> (global::Error, global::Error),
{
    let manual = Node::new_alloc();
    let refc = user_refc_instance();

    let manual2 = manual.clone();
    let refc2 = refc.clone();

    let (a, b) = f(manual, refc);

    assert_eq!(a, global::Error::OK);
    assert_eq!(b, global::Error::OK);
    assert_eq!(manual2.get_name(), "hello");
    assert_eq!(refc2.bind().value, -123);

    manual2.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests requiring codegen-full feature

#[cfg(feature = "codegen-full")]
mod engine_api_tests {
    use std::cell::Cell;

    use godot::builtin::{Rid, Variant};
    use godot::classes::{base_material_3d, ITexture2D, StandardMaterial3D, Texture2D};
    use godot::meta::ToGodot;
    use godot::obj::{Base, Gd, NewGd, WithBaseField};
    use godot::register::{godot_api, GodotClass};

    use crate::framework::itest;

    const ALBEDO: base_material_3d::TextureParam = base_material_3d::TextureParam::ALBEDO;

    /// Various internal references are created during `set_texture()`, thus 4. This also matches GDScript code doing the same.
    /// Verified that before this optimization, the refcount was 5.
    const EXPECTED_REFCOUNT: i32 = 4;

    fn verify_refcount<F>(exp_refcount: i32, arg_desription: &str, operation: F)
    where
        F: FnOnce(&mut Gd<StandardMaterial3D>, &Gd<ArgTestTexture>),
    {
        let texture = ArgTestTexture::new_gd();
        let mut material = StandardMaterial3D::new_gd();

        operation(&mut material, &texture);

        let captured = texture.bind().get_captured_refcount();
        assert_eq!(captured, exp_refcount, "{}", arg_desription);
    }

    #[itest]
    fn refcount_asarg_ref() {
        verify_refcount(EXPECTED_REFCOUNT, "&object", |mat, tex| {
            // Sanity check: refcount is 1 before call.
            assert_eq!(tex.get_reference_count(), 1);

            mat.set_texture(ALBEDO, tex);
        });

        // Derived -> base conversion. 1 extra due to clone().
        verify_refcount(EXPECTED_REFCOUNT + 1, "&base_obj", |mat, tex| {
            let base = tex.clone().upcast::<Texture2D>();
            mat.set_texture(ALBEDO, &base);
        });
    }

    #[itest]
    fn refcount_asarg_option() {
        verify_refcount(EXPECTED_REFCOUNT, "Some(&object)", |mat, tex| {
            mat.set_texture(ALBEDO, Some(tex));
        });

        // Derived -> base conversion. 1 extra due to clone().
        verify_refcount(EXPECTED_REFCOUNT + 1, "Some(&base_obj)", |mat, tex| {
            let base = tex.clone().upcast::<Texture2D>();
            mat.set_texture(ALBEDO, Some(&base));
        });

        verify_refcount(0, "None [derived]", |mat, _tex| {
            mat.set_texture(ALBEDO, None::<&Gd<ArgTestTexture>>);
        });

        verify_refcount(0, "None [base]", |mat, _tex| {
            mat.set_texture(ALBEDO, None::<&Gd<Texture2D>>);
        });
    }

    #[itest]
    fn refcount_asarg_null_arg() {
        verify_refcount(0, "Gd::null_arg()", |mat, _tex| {
            mat.set_texture(ALBEDO, Gd::null_arg());
        });
    }

    #[itest]
    fn refcount_asarg_variant() {
        verify_refcount(EXPECTED_REFCOUNT, "&Variant(tex)", |mat, tex| {
            mat.set("albedo_texture", &tex.to_variant());
        });

        verify_refcount(0, "&Variant(nil)", |mat, _tex| {
            mat.set("albedo_texture", &Variant::nil());
        });
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Test classes for AsArg testing

    #[derive(GodotClass)]
    #[class(base = Texture2D)]
    pub struct ArgTestTexture {
        base: Base<Texture2D>,
        captured_refcount: Cell<i32>,
        rid: Rid,
    }

    #[godot_api]
    impl ArgTestTexture {
        fn get_captured_refcount(&self) -> i32 {
            self.captured_refcount.get()
        }
    }

    #[godot_api]
    impl ITexture2D for ArgTestTexture {
        fn init(base: Base<Texture2D>) -> Self {
            Self {
                base,
                captured_refcount: Cell::new(0),
                rid: Rid::new(0),
            }
        }

        // Override this method because it's called by StandardMaterial3D::set_texture().
        // We use it as a hook to observe the reference count from the engine side (after passing through AsArg).
        fn get_rid(&self) -> Rid {
            self.captured_refcount
                .set(self.base().get_reference_count());

            self.rid
        }

        fn get_width(&self) -> i32 {
            1
        }

        fn get_height(&self) -> i32 {
            1
        }
    }
}
