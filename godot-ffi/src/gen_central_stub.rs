/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: this code is only used during unit tests, and may be out of sync with the engine's values.
// The concrete values matter much less than having a structure at all, to avoid thousands of upstream
// conditional compilation differentiations.

use crate::{ffi_methods, GDExtensionTypePtr, /*GDExtensionVariantPtr,*/ GodotFfi};
pub mod types {
    pub type OpaqueNil = crate::opaque::Opaque<0usize>;
    pub type OpaqueBool = crate::opaque::Opaque<1usize>;
    pub type OpaqueInt = crate::opaque::Opaque<8usize>;
    pub type OpaqueFloat = crate::opaque::Opaque<8usize>;
    pub type OpaqueString = crate::opaque::Opaque<8usize>;
    pub type OpaqueVector2 = crate::opaque::Opaque<8usize>;
    pub type OpaqueVector2i = crate::opaque::Opaque<8usize>;
    pub type OpaqueRect2 = crate::opaque::Opaque<16usize>;
    pub type OpaqueRect2i = crate::opaque::Opaque<16usize>;
    pub type OpaqueVector3 = crate::opaque::Opaque<12usize>;
    pub type OpaqueVector3i = crate::opaque::Opaque<12usize>;
    pub type OpaqueTransform2D = crate::opaque::Opaque<24usize>;
    pub type OpaqueVector4 = crate::opaque::Opaque<16usize>;
    pub type OpaqueVector4i = crate::opaque::Opaque<16usize>;
    pub type OpaquePlane = crate::opaque::Opaque<16usize>;
    pub type OpaqueQuaternion = crate::opaque::Opaque<16usize>;
    pub type OpaqueAABB = crate::opaque::Opaque<24usize>;
    pub type OpaqueBasis = crate::opaque::Opaque<36usize>;
    pub type OpaqueTransform3D = crate::opaque::Opaque<48usize>;
    pub type OpaqueProjection = crate::opaque::Opaque<64usize>;
    pub type OpaqueColor = crate::opaque::Opaque<16usize>;
    pub type OpaqueStringName = crate::opaque::Opaque<8usize>;
    pub type OpaqueNodePath = crate::opaque::Opaque<8usize>;
    pub type OpaqueRID = crate::opaque::Opaque<8usize>;
    pub type OpaqueObject = crate::opaque::Opaque<8usize>;
    pub type OpaqueCallable = crate::opaque::Opaque<16usize>;
    pub type OpaqueSignal = crate::opaque::Opaque<16usize>;
    pub type OpaqueDictionary = crate::opaque::Opaque<8usize>;
    pub type OpaqueArray = crate::opaque::Opaque<8usize>;
    pub type OpaquePackedByteArray = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedInt32Array = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedInt64Array = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedFloat32Array = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedFloat64Array = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedStringArray = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedVector2Array = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedVector3Array = crate::opaque::Opaque<16usize>;
    pub type OpaquePackedColorArray = crate::opaque::Opaque<16usize>;
    pub type OpaqueVariant = crate::opaque::Opaque<24usize>;
}
// pub struct GlobalMethodTable {}
// impl GlobalMethodTable {
//     pub(crate) unsafe fn new(interface: &crate::GDExtensionInterface) -> Self {
//         Self {}
//     }
// }
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(i32)]
pub enum VariantType {
    Nil = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    String = 4,
    Vector2 = 5,
    Vector2i = 6,
    Rect2 = 7,
    Rect2i = 8,
    Vector3 = 9,
    Vector3i = 10,
    Transform2D = 11,
    Vector4 = 12,
    Vector4i = 13,
    Plane = 14,
    Quaternion = 15,
    AABB = 16,
    Basis = 17,
    Transform3D = 18,
    Projection = 19,
    Color = 20,
    StringName = 21,
    NodePath = 22,
    RID = 23,
    Object = 24,
    Callable = 25,
    Signal = 26,
    Dictionary = 27,
    Array = 28,
    PackedByteArray = 29,
    PackedInt32Array = 30,
    PackedInt64Array = 31,
    PackedFloat32Array = 32,
    PackedFloat64Array = 33,
    PackedStringArray = 34,
    PackedVector2Array = 35,
    PackedVector3Array = 36,
    PackedColorArray = 37,
}
impl VariantType {
    #[doc(hidden)]
    pub fn from_sys(enumerator: crate::GDExtensionVariantType) -> Self {
        match enumerator {
            0 => Self::Nil,
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Float,
            4 => Self::String,
            5 => Self::Vector2,
            6 => Self::Vector2i,
            7 => Self::Rect2,
            8 => Self::Rect2i,
            9 => Self::Vector3,
            10 => Self::Vector3i,
            11 => Self::Transform2D,
            12 => Self::Vector4,
            13 => Self::Vector4i,
            14 => Self::Plane,
            15 => Self::Quaternion,
            16 => Self::AABB,
            17 => Self::Basis,
            18 => Self::Transform3D,
            19 => Self::Projection,
            20 => Self::Color,
            21 => Self::StringName,
            22 => Self::NodePath,
            23 => Self::RID,
            24 => Self::Object,
            25 => Self::Callable,
            26 => Self::Signal,
            27 => Self::Dictionary,
            28 => Self::Array,
            29 => Self::PackedByteArray,
            30 => Self::PackedInt32Array,
            31 => Self::PackedInt64Array,
            32 => Self::PackedFloat32Array,
            33 => Self::PackedFloat64Array,
            34 => Self::PackedStringArray,
            35 => Self::PackedVector2Array,
            36 => Self::PackedVector3Array,
            37 => Self::PackedColorArray,
            _ => unreachable!("invalid variant type {}", enumerator),
        }
    }
    #[doc(hidden)]
    pub fn sys(self) -> crate::GDExtensionVariantType {
        self as _
    }
}
impl GodotFfi for VariantType {
    ffi_methods! { type GDExtensionTypePtr = * mut Self ; .. }
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(i32)]
pub enum VariantOperator {
    Equal = 0,
    NotEqual = 1,
    Less = 2,
    LessEqual = 3,
    Greater = 4,
    GreaterEqual = 5,
    Add = 6,
    Subtract = 7,
    Multiply = 8,
    Divide = 9,
    Negate = 10,
    Positive = 11,
    Module = 12,
    Power = 13,
    ShiftLeft = 14,
    ShiftRight = 15,
    BitAnd = 16,
    BitOr = 17,
    BitXor = 18,
    BitNegate = 19,
    And = 20,
    Or = 21,
    Xor = 22,
    Not = 23,
    In = 24,
}
impl VariantOperator {
    #[doc(hidden)]
    pub fn from_sys(enumerator: crate::GDExtensionVariantOperator) -> Self {
        match enumerator {
            0 => Self::Equal,
            1 => Self::NotEqual,
            2 => Self::Less,
            3 => Self::LessEqual,
            4 => Self::Greater,
            5 => Self::GreaterEqual,
            6 => Self::Add,
            7 => Self::Subtract,
            8 => Self::Multiply,
            9 => Self::Divide,
            10 => Self::Negate,
            11 => Self::Positive,
            12 => Self::Module,
            13 => Self::Power,
            14 => Self::ShiftLeft,
            15 => Self::ShiftRight,
            16 => Self::BitAnd,
            17 => Self::BitOr,
            18 => Self::BitXor,
            19 => Self::BitNegate,
            20 => Self::And,
            21 => Self::Or,
            22 => Self::Xor,
            23 => Self::Not,
            24 => Self::In,
            _ => unreachable!("invalid variant operator {}", enumerator),
        }
    }
    #[doc(hidden)]
    pub fn sys(self) -> crate::GDExtensionVariantOperator {
        self as _
    }
}
impl GodotFfi for VariantOperator {
    ffi_methods! { type GDExtensionTypePtr = * mut Self ; .. }
}
