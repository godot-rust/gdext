/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use sys::interface_fn;

use crate::builtin::{meta::ClassName, StringName};

/// A constant named `name` with the value `value`.
pub struct IntegerConstant {
    name: StringName,
    value: i64,
}

impl IntegerConstant {
    pub fn new<T: TryInto<i64> + std::fmt::Debug + Copy>(name: StringName, value: T) -> Self {
        Self {
            name,
            value: value.try_into().ok().unwrap_or_else(|| {
                panic!("exported constant `{value:?}` must be representable as `i64`")
            }),
        }
    }

    fn register(&self, class_name: &ClassName, enum_name: &StringName, is_bitfield: bool) {
        unsafe {
            interface_fn!(classdb_register_extension_class_integer_constant)(
                sys::get_library(),
                class_name.string_sys(),
                enum_name.string_sys(),
                self.name.string_sys(),
                self.value,
                is_bitfield as sys::GDExtensionBool,
            );
        }
    }
}

/// Whether the constant should be interpreted as a single integer, an enum with several variants, or a
/// bitfield with several flags.
pub enum ConstantKind {
    Integer(IntegerConstant),
    Enum {
        name: StringName,
        enumerators: Vec<IntegerConstant>,
    },
    Bitfield {
        name: StringName,
        flags: Vec<IntegerConstant>,
    },
}

impl ConstantKind {
    fn register(&self, class_name: &ClassName) {
        match self {
            ConstantKind::Integer(integer) => {
                integer.register(class_name, &StringName::default(), false)
            }
            ConstantKind::Enum { name, enumerators } => {
                for enumerator in enumerators.iter() {
                    enumerator.register(class_name, name, false)
                }
            }
            ConstantKind::Bitfield { name, flags } => {
                for flag in flags.iter() {
                    flag.register(class_name, name, true)
                }
            }
        }
    }
}

/// All the info needed to export a constant to Godot.
pub struct ExportConstant {
    class_name: ClassName,
    kind: ConstantKind,
}

impl ExportConstant {
    pub fn new(class_name: ClassName, kind: ConstantKind) -> Self {
        Self { class_name, kind }
    }

    pub fn register(&self) {
        self.kind.register(&self.class_name)
    }
}
