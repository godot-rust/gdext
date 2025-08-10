/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())]
#![allow(clippy::non_minimal_cfg)]

use godot::classes::ClassDb;
use godot::prelude::*;
use godot::sys::static_assert;

use crate::framework::itest;

#[derive(GodotClass)]
#[class(no_init)]
struct HasConstants {}

#[godot_api]
impl HasConstants {
    #[constant]
    const A: i64 = 128;

    #[constant]
    const B: i128 = -600;

    #[constant]
    const C: u8 = u8::MAX;

    #[constant]
    const D: usize = 20 + 33 * 45;

    #[constant]
    #[rustfmt::skip]
    const DONT_PANIC_WITH_SEGMENTED_PATH_ATTRIBUTE: bool = true;

    #[cfg(all())]
    #[constant]
    const CONSTANT_RECOGNIZED_WITH_SIMPLE_PATH_ATTRIBUTE_ABOVE_CONST_ATTR: bool = true;

    #[constant]
    #[cfg(all())]
    const CONSTANT_RECOGNIZED_WITH_SIMPLE_PATH_ATTRIBUTE_BELOW_CONST_ATTR: bool = true;

    // The three identically-named definitions below should be mutually exclusive thanks to #[cfg].
    #[constant]
    const CFG_REMOVES_DUPLICATE_CONSTANT_DEF: i64 = 5;

    #[cfg(any())]
    #[constant]
    const CFG_REMOVES_DUPLICATE_CONSTANT_DEF: i64 = compile_error!("Removed by #[cfg]");

    #[constant]
    #[cfg(any())]
    const CFG_REMOVES_DUPLICATE_CONSTANT_DEF: i64 = compile_error!("Removed by #[cfg]");

    // The constant below should end up not being defined at all.
    #[cfg(any())]
    #[constant]
    const CFG_REMOVES_CONSTANT: bool = compile_error!("Removed by #[cfg]");

    #[constant]
    #[cfg(any())]
    const CFG_REMOVES_CONSTANT: bool = compile_error!("Removed by #[cfg]");
}

/// Checks at runtime if a class has a given integer constant through [ClassDb].
fn class_has_integer_constant<T: GodotClass>(name: &str) -> bool {
    ClassDb::singleton().class_has_integer_constant(&T::class_name().to_string_name(), name)
}

#[itest]
fn constants_correct_value() {
    const CONSTANTS: [(&str, i64); 5] = [
        ("A", HasConstants::A),
        ("B", HasConstants::B as i64),
        ("C", HasConstants::C as i64),
        ("D", HasConstants::D as i64),
        (
            "CFG_REMOVES_DUPLICATE_CONSTANT_DEF",
            HasConstants::CFG_REMOVES_DUPLICATE_CONSTANT_DEF,
        ),
    ];

    let class_name = HasConstants::class_name().to_string_name();
    let constants = ClassDb::singleton()
        .class_get_integer_constant_list_ex(&class_name)
        .no_inheritance(true)
        .done();

    for (constant_name, constant_value) in CONSTANTS {
        assert!(constants.contains(constant_name));
        assert_eq!(
            ClassDb::singleton().class_get_integer_constant(&class_name, constant_name),
            constant_value
        );
    }

    // Ensure the constants are still present and are equal to 'true'
    static_assert!(HasConstants::CONSTANT_RECOGNIZED_WITH_SIMPLE_PATH_ATTRIBUTE_ABOVE_CONST_ATTR);
    static_assert!(HasConstants::CONSTANT_RECOGNIZED_WITH_SIMPLE_PATH_ATTRIBUTE_BELOW_CONST_ATTR);
}

#[itest]
fn cfg_removes_or_keeps_constants() {
    assert!(class_has_integer_constant::<HasConstants>(
        "CFG_REMOVES_DUPLICATE_CONSTANT_DEF"
    ));
    assert!(!class_has_integer_constant::<HasConstants>(
        "CFG_REMOVES_CONSTANT"
    ));
}

#[derive(GodotClass)]
#[class(no_init)]
struct HasOtherConstants {}

impl HasOtherConstants {
    const ENUM_NAME: &'static str = "SomeEnum";
    const ENUM_A: i64 = 0;
    const ENUM_B: i64 = 1;
    const ENUM_C: i64 = 2;

    const BITFIELD_NAME: &'static str = "SomeBitfield";
    const BITFIELD_A: i64 = 1;
    const BITFIELD_B: i64 = 2;
    const BITFIELD_C: i64 = 4;
}

// TODO: replace with proc-macro api when constant enums and bitfields can be exported through the
// proc-macro.
impl godot::obj::cap::ImplementsGodotApi for HasOtherConstants {
    fn __register_methods() {}
    fn __register_constants() {
        use ::godot::register::private::constant::*;
        // Try exporting an enum.
        ExportConstant::new(
            HasOtherConstants::class_name(),
            ConstantKind::Enum {
                name: Self::ENUM_NAME.into(),
                enumerators: vec![
                    IntegerConstant::new("ENUM_A", Self::ENUM_A),
                    IntegerConstant::new("ENUM_B", Self::ENUM_B),
                    IntegerConstant::new("ENUM_C", Self::ENUM_C),
                ],
            },
        )
        .register();

        // Try exporting an enum.
        ExportConstant::new(
            HasOtherConstants::class_name(),
            ConstantKind::Bitfield {
                name: Self::BITFIELD_NAME.into(),
                flags: vec![
                    IntegerConstant::new("BITFIELD_A", Self::BITFIELD_A),
                    IntegerConstant::new("BITFIELD_B", Self::BITFIELD_B),
                    IntegerConstant::new("BITFIELD_C", Self::BITFIELD_C),
                ],
            },
        )
        .register();
    }
}

// TODO once this is done via proc-macro, see if `register-docs` is still used in register_docs_test.rs. Otherwise, remove feature from Cargo.toml.
godot::sys::plugin_add!(
    godot::private::__GODOT_PLUGIN_REGISTRY;
    godot::private::ClassPlugin::new::<HasOtherConstants>(
        godot::private::PluginItem::InherentImpl(
            godot::private::InherentImpl::new::<HasOtherConstants>(
                #[cfg(feature = "register-docs")]
                Default::default()
            )
        )
    )
);

macro_rules! test_enum_export {
    (
        $class:ty, $enum_name:ident, [$($enumerators:ident),* $(,)?];
        // Include the `attr` here too, so we can easily do things like `#[itest (focus)]`.
        #$attr:tt
        fn $test_name:ident() { .. }
    ) => {
        #$attr
        fn $test_name() {
            let class_name = <$class>::class_name().to_string_name();
            let enum_name = StringName::from(<$class>::$enum_name);
            let variants = [
                $((stringify!($enumerators), <$class>::$enumerators)),*
            ];

            assert!(ClassDb::singleton()
                .class_has_enum_ex(&class_name, &enum_name)
                .no_inheritance(true)
                .done());

            let godot_variants = ClassDb::singleton()
                .class_get_enum_constants_ex(&class_name, &enum_name)
                .no_inheritance(true)
                .done();

            let constants = ClassDb::singleton()
                .class_get_integer_constant_list_ex(&class_name)
                .no_inheritance(true)
                .done();

            for (variant_name, variant_value) in variants {
                let variant_name = GString::from(variant_name);
                assert!(godot_variants.contains(&variant_name));
                assert!(constants.contains(&variant_name));
                assert_eq!(
                    ClassDb::singleton().class_get_integer_constant(&class_name, variant_name.arg()),
                    variant_value
                );
            }
        }
    }
}

test_enum_export!(
    HasOtherConstants, ENUM_NAME, [ENUM_A, ENUM_B, ENUM_C];
    #[itest]
    fn enum_export_correct_values() { .. }
);

test_enum_export!(
    HasOtherConstants, BITFIELD_NAME, [BITFIELD_A, BITFIELD_B, BITFIELD_C];
    #[itest]
    fn bitfield_export_correct_values() { .. }
);
