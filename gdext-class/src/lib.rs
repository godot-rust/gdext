mod obj;
mod registry;
mod storage;
mod traits;

pub mod macros;
pub mod property_info;

pub use obj::*;
pub use registry::*;
pub use traits::*;

use gdext_sys as sys;

#[doc(hidden)]
pub mod private {
    pub use crate::storage::as_storage;
}
