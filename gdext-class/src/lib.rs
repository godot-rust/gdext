mod as_arg;
mod obj;
mod registry;
mod storage;
mod traits;

pub mod macros;
pub mod property_info;

pub use as_arg::*;
pub use obj::*;
pub use registry::*;
pub use traits::*;

use gdext_sys as sys;

mod gen {
    pub(crate) mod classes;
}

pub mod api {
    pub use super::gen::classes::*;
}

#[doc(hidden)]
pub mod private {
    pub use crate::storage::as_storage;
}

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! out {
    ()                          => (println!());
    ($fmt:literal)              => (println!($fmt));
    ($fmt:literal, $($arg:tt)*) => (println!($fmt, $($arg)*);)
}

#[cfg(not(feature = "trace"))]
// TODO find a better way than sink-writing to avoid warnings, #[allow(unused_variables)] doesn't work
#[macro_export]
macro_rules! out {
    ()                          => ({});
    ($fmt:literal)              => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt); });
    ($fmt:literal, $($arg:tt)*) => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt, $($arg)*); };)
}
