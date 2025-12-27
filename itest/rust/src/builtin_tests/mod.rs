/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod geometry {
    mod aabb_test;
    mod basis_test;
    mod plane_test;
    mod projection_test;
    mod quaternion_test;
    mod rect2_test;
    mod rect2i_test;
    mod transform2d_test;
    mod transform3d_test;
    mod vector_test {
        mod vector2_test;
        mod vector2i_test;
        mod vector3_test;
        mod vector3i_test;
        mod vector4_test;
        mod vector4i_test;
    }
}

mod containers {
    mod array_test;
    mod callable_test;
    mod dictionary_test;
    mod packed_array_test;
    mod rid_test;
    mod signal_disconnect_test;
    mod signal_test;
    mod variant_test;
}

mod string {
    mod gstring_test;
    mod node_path_test;
    mod string_name_test;
    mod string_test_macros;
}

mod script {
    mod script_instance_tests;
}

mod color_test;

mod convert_test;

mod common;

#[cfg(feature = "serde")]
mod serde_test;
