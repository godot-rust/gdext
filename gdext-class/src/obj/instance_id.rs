use gdext_builtin::Variant;
use gdext_sys::{self as sys, ffi_methods, GodotFfi};
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Represents an instance ID.
///
/// This is its own type for type safety and to deal with the inconsistent representation in Godot as both `u64` (C++) and `i64` (GDScript).
/// You can usually treat this as an opaque value and pass it to and from GDScript; there are conversion methods however.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct InstanceId {
    value: u64,
}

impl InstanceId {
    pub fn from_u64(id: u64) -> Self {
        InstanceId { value: id }
    }

    pub fn from_i64(id: i64) -> Self {
        InstanceId { value: id as u64 }
    }

    pub fn to_u64(self) -> u64 {
        self.value
    }

    pub fn to_i64(self) -> i64 {
        self.value as i64
    }

    /// Returns if the object being referred-to is inheriting `RefCounted`
    #[allow(dead_code)]
    pub(crate) fn is_ref_counted(self) -> bool {
        self.value & (1u64 << 63) != 0
    }
}

impl Display for InstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:x}", self.value)
    }
}

impl GodotFfi for InstanceId {
    ffi_methods! { type sys::GDNativeTypePtr = *mut Self; .. }
}

impl From<&Variant> for InstanceId {
    fn from(variant: &Variant) -> Self {
        let int = i64::from(variant);
        InstanceId::from_i64(int)
    }
}

impl From<Variant> for InstanceId {
    fn from(variant: Variant) -> Self {
        InstanceId::from(&variant)
    }
}

impl From<InstanceId> for Variant {
    fn from(instance_id: InstanceId) -> Self {
        let int = instance_id.to_i64();
        Variant::from(int)
    }
}
