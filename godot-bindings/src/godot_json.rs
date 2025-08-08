/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Commands related to parsing user-provided JSON and extension headers.

// At first re-using mapping from godot-codegen json.rs might seem more desirable but there are few issues to consider:
// * Overall JSON file structure might change slightly from version to version, while header should stay consistent (otherwise it defeats the purpose of having any header at all).
// Having two parsers – minimal one inherent to api-custom-json feature and codegen one – makes handling all the edge cases easier.
// * `godot-codegen` depends on `godot-bindings` thus simple re-using types from former in side the latter is not possible (cyclic dependency).
// Moving said types to `godot-bindings` would increase the cognitive overhead (since domain mapping is responsibility of `godot-codegen`, while godot-bindings is responsible for providing required resources & emitting the version).
// In the future we might experiment with splitting said types into separate crates.

use std::fs;
use std::path::Path;

use nanoserde::DeJson;

use crate::depend_on_custom_json::header_gen::generate_rust_binding;
use crate::godot_version::validate_godot_version;
use crate::{GodotVersion, StopWatch};

#[rustfmt::skip] // Do not reorder.
// GDExtension headers are backward compatible (new incremental changes in general are exposed as additions to the existing API) while godot-rust simply ignores extra declarations in header file.
// Therefore, latest headers should work fine for all the past and future Godot versions – as long as the engine remains unchanged.
// [version-sync] [[
//  [include] current.minor
//  [line] use gdextension_api::version_$snakeVersion::load_gdextension_header_h as load_latest_gdextension_headers;
use gdextension_api::version_4_4::load_gdextension_header_h as load_latest_gdextension_headers;
// ]]

/// A minimal version of deserialized JsonExtensionApi that includes only the header.
#[derive(DeJson)]
struct JsonExtensionApi {
    pub header: JsonHeader,
}

/// Deserialized "header" key in given `extension_api.json`.
#[derive(DeJson)]
struct JsonHeader {
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    pub version_status: String,
    pub version_build: String,
    pub version_full_name: String,
}

impl JsonHeader {
    fn into_godot_version(self) -> GodotVersion {
        GodotVersion {
            full_string: self.version_full_name,
            major: self.version_major,
            minor: self.version_minor,
            patch: self.version_patch,
            status: self.version_status,
            custom_rev: Some(self.version_build),
        }
    }
}

pub fn load_custom_gdextension_json() -> String {
    let path = std::env::var("GODOT4_GDEXTENSION_JSON").expect(
        "godot-rust with `api-custom-json` feature requires GODOT4_GDEXTENSION_JSON \
        environment variable (with the path to the said json).",
    );
    let json_path = Path::new(&path);

    fs::read_to_string(json_path).unwrap_or_else(|_| {
        panic!(
            "failed to open file with custom GDExtension JSON {}.",
            json_path.display()
        )
    })
}

pub(crate) fn read_godot_version() -> GodotVersion {
    let extension_api: JsonExtensionApi = DeJson::deserialize_json(&load_custom_gdextension_json())
        .expect("failed to deserialize JSON");
    let version = extension_api.header.into_godot_version();

    validate_godot_version(&version);

    version
}

pub(crate) fn write_gdextension_headers(
    out_h_path: &Path,
    out_rs_path: &Path,
    watch: &mut StopWatch,
) {
    let h_contents = load_latest_gdextension_headers();
    fs::write(out_h_path, h_contents.as_ref())
        .unwrap_or_else(|e| panic!("failed to write gdextension_interface.h: {e}"));
    watch.record("write_header_h");

    generate_rust_binding(out_h_path, out_rs_path);
    watch.record("generate_header_rs");
}
