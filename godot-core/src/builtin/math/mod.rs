/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod approx_eq;
mod float;
mod glam_helpers;
mod xform;

pub use approx_eq::ApproxEq;
pub use float::FloatExt;
// Internal glam re-exports
pub(crate) use glam_helpers::*;
pub use xform::XformInv;

pub use crate::{assert_eq_approx, assert_ne_approx};

#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
mod test {
    use super::*;

    #[test]
    fn equal_approx() {
        assert_eq_approx!(1.0, 1.000001);
        assert_ne_approx!(1.0, 2.0);
        assert_eq_approx!(1.0, 1.000001, "Message {}", "formatted");
        assert_ne_approx!(1.0, 2.0, "Message {}", "formatted");
    }
}
