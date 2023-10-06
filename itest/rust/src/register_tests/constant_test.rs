/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())]
#![allow(clippy::non_minimal_cfg)]

use crate::framework::itest;
use godot::engine::ClassDb;
use godot::prelude::*;
use godot::sys::static_assert;

#[derive(GodotClass)]
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
}

#[itest]
fn constants_correct_value() {
    const CONSTANTS: [(&str, i64); 4] = [
        ("A", HasConstants::A),
        ("B", HasConstants::B as i64),
        ("C", HasConstants::C as i64),
        ("D", HasConstants::D as i64),
    ];

    let constants = ClassDb::singleton()
        .class_get_integer_constant_list_ex(HasConstants::class_name().to_string_name())
        .no_inheritance(true)
        .done();

    for (constant_name, constant_value) in CONSTANTS {
        assert!(constants.contains(constant_name.into()));
        assert_eq!(
            ClassDb::singleton().class_get_integer_constant(
                HasConstants::class_name().to_string_name(),
                constant_name.into()
            ),
            constant_value
        );
    }

    // Ensure the constants are still present and are equal to 'true'
    static_assert!(HasConstants::CONSTANT_RECOGNIZED_WITH_SIMPLE_PATH_ATTRIBUTE_ABOVE_CONST_ATTR);
    static_assert!(HasConstants::CONSTANT_RECOGNIZED_WITH_SIMPLE_PATH_ATTRIBUTE_BELOW_CONST_ATTR);
}

#[derive(GodotClass)]
struct HasOtherConstants {}

impl HasOtherConstants {
    const ENUM_NAME: &str = "SomeEnum";
    const ENUM_A: i64 = 0;
    const ENUM_B: i64 = 1;
    const ENUM_C: i64 = 2;

    const BITFIELD_NAME: &str = "SomeBitfield";
    const BITFIELD_A: i64 = 1;
    const BITFIELD_B: i64 = 2;
    const BITFIELD_C: i64 = 4;
}

// TODO: replace with proc-macro api when constant enums and bitfields can be exported through the
// proc-macro.
impl godot::obj::cap::ImplementsGodotApi for HasOtherConstants {
    fn __register_methods() {}
    fn __register_constants() {
        use ::godot::builtin::meta::registration::constant::*;
        // Try exporting an enum.
        ExportConstant::new(
            HasOtherConstants::class_name(),
            ConstantKind::Enum {
                name: Self::ENUM_NAME.into(),
                enumerators: vec![
                    IntegerConstant::new("ENUM_A".into(), Self::ENUM_A),
                    IntegerConstant::new("ENUM_B".into(), Self::ENUM_B),
                    IntegerConstant::new("ENUM_C".into(), Self::ENUM_C),
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
                    IntegerConstant::new("BITFIELD_A".into(), Self::BITFIELD_A),
                    IntegerConstant::new("BITFIELD_B".into(), Self::BITFIELD_B),
                    IntegerConstant::new("BITFIELD_C".into(), Self::BITFIELD_C),
                ],
            },
        )
        .register();
    }
}

godot::sys::plugin_add!(
    __GODOT_PLUGIN_REGISTRY in ::godot::private;
    ::godot::private::ClassPlugin {
        class_name: HasOtherConstants::class_name(),
        component: ::godot::private::PluginComponent::UserMethodBinds {
            generated_register_fn: ::godot::private::ErasedRegisterFn {
                raw: ::godot::private::callbacks::register_user_binds::<HasOtherConstants>,
            },
        },
        init_level: HasOtherConstants::INIT_LEVEL,
    }
);

macro_rules! test_enum_export {
    (
        $class:ty, $enum_name:ident, [$($enumerators:ident),* $(,)?];
        // Include the `attr` here to so we can easily do things like `#[itest(focus)]`.
        #$attr:tt
        fn $test_name:ident() { .. }
    ) => {
        #$attr
        fn $test_name() {
            let class_name = <$class>::class_name();
            let enum_name = StringName::from(<$class>::$enum_name);
            let variants = [
                $((stringify!($enumerators), <$class>::$enumerators)),*
            ];

            assert!(ClassDb::singleton()
                .class_has_enum_ex(
                    class_name.to_string_name(),
                    enum_name.clone(),
                )
                .no_inheritance(true)
                .done());

            let godot_variants = ClassDb::singleton()
                .class_get_enum_constants_ex(
                    class_name.to_string_name(),
                    enum_name.into(),
                )
                .no_inheritance(true)
                .done();

            let constants = ClassDb::singleton()
                .class_get_integer_constant_list_ex(class_name.to_string_name())
                .no_inheritance(true)
                .done();

            for (variant_name, variant_value) in variants {
                assert!(godot_variants.contains(variant_name.into()));
                assert!(constants.contains(variant_name.into()));
                assert_eq!(
                    ClassDb::singleton()
                        .class_get_integer_constant(class_name.to_string_name(), variant_name.into()),
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
