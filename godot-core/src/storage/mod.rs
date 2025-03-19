/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod instance_storage;
#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
mod multi_threaded;
#[cfg_attr(feature = "experimental-threads", allow(dead_code))]
mod single_threaded;

use godot_ffi::out;
pub use instance_storage::*;
use std::any::type_name;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared code for submodules

fn bind_failed<T>(err: Box<dyn std::error::Error>) -> ! {
    let ty = type_name::<T>();
    panic!(
        "Gd<T>::bind() failed, already bound; T = {ty}.\n  \
        Make sure to use `self.base_mut()` or `self.base()` instead of `self.to_gd()` when possible.\n  \
        Details: {err}."
    )
}

fn bind_mut_failed<T>(err: Box<dyn std::error::Error>) -> ! {
    let ty = type_name::<T>();
    panic!(
        "Gd<T>::bind_mut() failed, already bound; T = {ty}.\n  \
        Make sure to use `self.base_mut()` instead of `self.to_gd()` when possible.\n  \
        Details: {err}."
    )
}

fn bug_inaccessible<T>(err: Box<dyn std::error::Error>) -> ! {
    // We should never hit this, except maybe in extreme cases like having more than
    // `usize::MAX` borrows.
    let ty = type_name::<T>();
    panic!(
        "`base_mut()` failed for type T = {ty}.\n  \
        This is most likely a bug, please report it.\n  \
        Details: {err}."
    )
}

fn log_construct<T>() {
    out!(
        "    Storage::construct             <{ty}>",
        ty = type_name::<T>()
    );
}

fn log_inc_ref<T: StorageRefCounted>(storage: &T) {
    out!(
        "    Storage::on_inc_ref (rc={rc})     <{ty}> -- {base:?}",
        rc = T::godot_ref_count(storage),
        base = storage.base(),
        ty = type_name::<T>(),
    );
}

fn log_dec_ref<T: StorageRefCounted>(storage: &T) {
    out!(
        "  | Storage::on_dec_ref (rc={rc})     <{ty}> -- {base:?}",
        rc = T::godot_ref_count(storage),
        base = storage.base(),
        ty = type_name::<T>(),
    );
}

fn log_drop<T: StorageRefCounted>(storage: &T) {
    out!(
        "    Storage::drop (rc={rc})           <{base:?}>",
        rc = storage.godot_ref_count(),
        base = storage.base(),
    );
}
