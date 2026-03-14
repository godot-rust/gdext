/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Advanced conversion machinery.

pub use super::args::{ArgPassing, AsDirectElement, ByObject, ByOption, ByRef, ByValue, ByVariant};
pub use super::object_to_owned::ObjectToOwned;
pub use super::param_tuple::{InParamTuple, OutParamTuple, ParamTuple, TupleFromGodot};
pub use super::raw_ptr::{FfiRawPointer, RawPtr};
pub use super::uniform_object_deref::UniformObjectDeref;
