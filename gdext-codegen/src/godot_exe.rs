use std::path::{Path, PathBuf};
use std::process::Command;

/// Commands related to Godot executable

const EXTENSION_API_PATH: &'static str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/input/extension_api.json");

pub fn load_extension_api_json() -> String {
    let path = Path::new(EXTENSION_API_PATH);
    rerun_on_changed(path);

    match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(_) => {
            dump_extension_api(path);
            std::fs::read_to_string(path).expect(&format!("failed to open file {:?}", path))
        }
    }
}

fn dump_extension_api(path: &Path) {
    let cwd = path.parent().unwrap();
    let godot_bin = locate_godot_binary();
    rerun_on_changed(&godot_bin);

    Command::new(&godot_bin)
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
