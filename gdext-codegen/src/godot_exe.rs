use crate::godot_version::parse_godot_version;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Commands related to Godot executable

const GODOT_VERSION_PATH: &'static str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/input/godot_version.txt");

const EXTENSION_API_PATH: &'static str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/input/extension_api.json");

pub fn load_extension_api_json() -> String {
    let json_path = Path::new(EXTENSION_API_PATH);
    rerun_on_changed(json_path);

    let godot_bin = locate_godot_binary();
    rerun_on_changed(&godot_bin);

    // Regnerate API JSON if first time or Godot version is different
    if !json_path.exists() || has_version_changed(&godot_bin) {
        dump_extension_api(&godot_bin, json_path);
    }

    std::fs::read_to_string(json_path)
        .expect(&format!("failed to open file {}", json_path.display()))
}

fn has_version_changed(godot_bin: &Path) -> bool {
    let version_path = Path::new(GODOT_VERSION_PATH);
    rerun_on_changed(version_path);

    let current_version = read_godot_version(&godot_bin);
    let changed = match std::fs::read_to_string(version_path) {
        Ok(last_version) => current_version != last_version,
        Err(_) => true,
    };

    if changed {
        std::fs::write(version_path, current_version).expect(&format!(
            "write Godot version to file {}",
            version_path.display()
        ));
    }
    changed
}

fn read_godot_version(godot_bin: &Path) -> String {
    let output = Command::new(&godot_bin)
        .arg("--version")
        .output()
        .expect(&format!(
            "failed to invoke Godot executable '{}'",
            godot_bin.display()
        ));

    let output = String::from_utf8(output.stdout).expect("convert Godot version to UTF-8");

    match parse_godot_version(&output) {
        Ok(parsed) => {
            assert!(
                parsed.major == 4,
                "Only Godot versions >= 4.0 are supported; found version {}.",
                output.trim()
            );

            parsed.full_string
        }
        Err(e) => {
            // Don't treat this as fatal error
            panic!("failed to parse Godot version '{}': {}", output, e)
        }
    }
}

fn dump_extension_api(godot_bin: &Path, out_file: &Path) {
    let cwd = out_file.parent().unwrap();

    Command::new(godot_bin)
        .current_dir(cwd)
        .arg("--no-window")
        .arg("--dump-extension-api")
        .arg(cwd)
        .output()
        .expect(&format!(
            "failed to invoke Godot executable '{}'",
            godot_bin.display()
        ));
}

fn locate_godot_binary() -> PathBuf {
    if let Ok(string) = std::env::var("GODOT_BIN") {
        println!("Found GODOT_BIN with path to executable: '{}'", string);
        PathBuf::from(string)
    } else if let Ok(path) = which::which("godot4") {
        println!("Found 'godot4' executable in PATH: {}", path.display());
        path
    } else {
        panic!(
            "Bindings generation requires 'godot4' executable or a GODOT_BIN \
                 environment variable (with the path to the executable)."
        )
    }
}

fn rerun_on_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}
