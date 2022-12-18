/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::godot_version::parse_godot_version;
use crate::StopWatch;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Commands related to Godot executable

const GODOT_VERSION_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/input/gen/godot_version.txt");

const EXTENSION_API_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/input/gen/extension_api.json");

pub fn load_extension_api_json(watch: &mut StopWatch) -> String {
    let json_path = Path::new(EXTENSION_API_PATH);
    rerun_on_changed(json_path);

    let godot_bin = locate_godot_binary();
    rerun_on_changed(&godot_bin);
    watch.record("locate_godot");

    // Regenerate API JSON if first time or Godot version is different
    let version = read_godot_version(&godot_bin);
    if !json_path.exists() || has_version_changed(&version) {
        dump_extension_api(&godot_bin, json_path);
        update_version_file(&version);

        watch.record("dump_extension_api");
    }

    let result = std::fs::read_to_string(json_path)
        .unwrap_or_else(|_| panic!("failed to open file {}", json_path.display()));
    watch.record("read_json_file");
    result
}

fn has_version_changed(current_version: &str) -> bool {
    let version_path = Path::new(GODOT_VERSION_PATH);

    match std::fs::read_to_string(version_path) {
        Ok(last_version) => current_version != last_version,
        Err(_) => true,
    }
}

fn update_version_file(version: &str) {
    let version_path = Path::new(GODOT_VERSION_PATH);
    rerun_on_changed(version_path);

    std::fs::write(version_path, version)
        .unwrap_or_else(|_| panic!("write Godot version to file {}", version_path.display()));
}

fn read_godot_version(godot_bin: &Path) -> String {
    let output = Command::new(godot_bin)
        .arg("--version")
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "failed to invoke Godot executable '{}'",
                godot_bin.display()
            )
        });

    let output = String::from_utf8(output.stdout).expect("convert Godot version to UTF-8");
    println!("Godot version: {output}");

    match parse_godot_version(&output) {
        Ok(parsed) => {
            assert_eq!(
                parsed.major,
                4,
                "Only Godot versions >= 4.0 are supported; found version {}.",
                output.trim()
            );

            parsed.full_string
        }
        Err(e) => {
            // Don't treat this as fatal error
            panic!("failed to parse Godot version '{output}': {e}")
        }
    }
}

fn dump_extension_api(godot_bin: &Path, out_file: &Path) {
    let cwd = out_file.parent().unwrap();
    std::fs::create_dir_all(cwd).unwrap_or_else(|_| panic!("create directory '{}'", cwd.display()));
    println!("Dump extension API to dir '{}'...", cwd.display());

    Command::new(godot_bin)
        .current_dir(cwd)
        .arg("--headless")
        .arg("--dump-extension-api")
        .arg(cwd)
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "failed to invoke Godot executable '{}'",
                godot_bin.display()
            )
        });

    println!("Generated {}/extension_api.json.", cwd.display());
}

fn locate_godot_binary() -> PathBuf {
    if let Ok(string) = std::env::var("GODOT4_BIN") {
        println!("Found GODOT4_BIN with path to executable: '{string}'");
        PathBuf::from(string)
    } else if let Ok(path) = which::which("godot4") {
        println!("Found 'godot4' executable in PATH: {}", path.display());
        path
    } else {
        panic!(
            "Bindings generation requires 'godot4' executable or a GODOT4_BIN \
                 environment variable (with the path to the executable)."
        )
    }
}

fn rerun_on_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}
