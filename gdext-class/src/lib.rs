pub mod obj;
mod registry;
mod storage;

pub mod builder;
pub mod macros;
pub mod property_info;
pub mod traits;

pub use registry::*;

use gdext_sys as sys;

mod gen {
    #[allow(unused_imports)]
    pub(crate) mod classes;
    pub mod utilities;
}

pub mod api {
    pub use super::gen::classes::*;
    pub use super::gen::utilities;
}

#[doc(hidden)]
pub mod private {
    pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginComponent};
    pub use crate::storage::as_storage;

    gdext_sys::plugin_registry!(GDEXT_CLASS_REGISTRY: ClassPlugin);

    pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
        gdext_sys::plugin_foreach!(GDEXT_CLASS_REGISTRY; visitor);
    }
}

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! out {
    ()                          => (eprintln!());
    ($fmt:literal)              => (eprintln!($fmt));
    ($fmt:literal, $($arg:tt)*) => (eprintln!($fmt, $($arg)*));
}

#[cfg(not(feature = "trace"))]
// TODO find a better way than sink-writing to avoid warnings, #[allow(unused_variables)] doesn't work
#[macro_export]
macro_rules! out {
    ()                          => ({});
    ($fmt:literal)              => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt); });
    ($fmt:literal, $($arg:tt)*) => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt, $($arg)*); };)
}
