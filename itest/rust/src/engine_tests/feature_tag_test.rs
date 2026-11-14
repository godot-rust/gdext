/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::Os;
use godot::obj::Singleton;
use godot::tools::{FeatureTag, has_feature};

use crate::framework::itest;

#[itest]
fn feature_tag_agrees_with_os() {
    // The integration tests run inside the editor, so this tag must match what the engine reports for the same string.
    assert_eq!(
        has_feature(&FeatureTag::EDITOR_HINT),
        Os::singleton().has_feature("editor_hint")
    );
    assert_eq!(
        has_feature(&FeatureTag::THREADS),
        Os::singleton().has_feature("threads")
    );
}

#[itest]
fn feature_tag_precision_is_exclusive() {
    // A build is either single or double precision, never both.
    assert_ne!(
        has_feature(&FeatureTag::SINGLE),
        has_feature(&FeatureTag::DOUBLE)
    );
}

#[itest]
fn feature_tag_user_defined_is_passed_through() {
    // No export preset declares this one, so the engine reports it as missing.
    assert!(!has_feature(&FeatureTag::user_defined(
        "gdext_itest_undefined_feature"
    )));
}
