## This script is used to create a new Godot project with Rust support or add Rust support to an existing project.
## To run the script, navigate to the project folder and run the following command:
##
##     curl https://raw.githubusercontent.com/godot-rust/gdext/refs/heads/master/bin/new.sh > ./tmp-new.sh && ./tmp-new.sh && rm ./tmp-new.sh
##
## If the script is run in a folder with a ".godot" folder, it will ask if you want
## to add Rust support to the current project or restructure it.
##   If the user chooses to restructure the project, the script will move the files
##   to a temporary folder then restructure the project as follows
##   (where "root" is the name of the project folder):
##
##     - "root"
##       - "godot"
##         - ".godot"
##         - "rust.gdextension"
##       - "rust"
##         - "src"
##           - "lib.rs"
##         - "Cargo.toml"
##
##   Otherwise, it will add a rust folder to the current project and add a "rust"
##   folder in the current directory with the following structure
##   (where "root" is the name of the project folder):
##
##     - "root"
##       - ".godot"
##       - "rust"
##         - "src"
##           - "lib.rs"
##         - "Cargo.toml"
##       - "rust.gdextension"
##
##
## If the script is run in a folder without a ".godot" folder, it will ask the
## user for a project name and create a new project with with a structure described
## above in the "restructure" section.
##

# The the current directory that the script was initiated from
CURRENT_DIR=$(pwd)

function create_rust_project {
  ROOT_DIR=$1
  PROJECT_NAME=$2

  # Start from a fresh location
  cd $ROOT_DIR

  echo -e "\e[38;5;117mCreating new Rust project \"$PROJECT_NAME/rust\"\e[0m"
  cargo new rust --lib

  cd rust
  echo -e "\e[38;5;117mAdding resolver = \"2\" to the Cargo.toml file\e[0m"
  sed -i '/\[package\]/aresolver = "2"' Cargo.toml

  sed -i '$a\
\n[lib]\
crate-type = ["cdylib"]\n\
[profile.dev]\
opt-level = 0\n\
[profile.dev.package."*"]\
opt-level = 3' Cargo.toml

  echo -e "\e[38;5;117mAdding godot crate to the Cargo.toml file\e[0m"
  cargo add godot
  # cargo build
}

function create_gdextension {
  ROOT_DIR=$1
  PROJECT_NAME=$2
  RUST_DIR=$3

  # Start from the godot project folder
  cd $ROOT_DIR


  echo -e "\e[38;5;117mCreating GDExtension: \"rust.gdextension\"\e[0m"
  touch rust.gdextension

  echo -e "\e[38;5;117mAdding configuration to \"rust.gdextension\"\e[0m"
  echo "[configuration]
entry_symbol = \"gdext_rust_init\"
compatibility_minimum = 4.1
reloadable = true

[libraries]
linux.debug.x86_64 =     \"res://$RUST_DIR/target/debug/lib$PROJECT_NAME.so\"
linux.release.x86_64 =   \"res://$RUST_DIR/target/release/lib$PROJECT_NAME.so\"
windows.debug.x86_64 =   \"res://$RUST_DIR/target/debug/$PROJECT_NAME.dll\"
windows.release.x86_64 = \"res://$RUST_DIR/target/release/$PROJECT_NAME.dll\"
macos.debug =            \"res://$RUST_DIR/target/debug/lib$PROJECT_NAME.dylib\"
macos.release =          \"res://$RUST_DIR/target/release/lib$PROJECT_NAME.dylib\"
macos.debug.arm64 =      \"res://$RUST_DIR/target/debug/lib$PROJECT_NAME.dylib\"
macos.release.arm64 =    \"res://$RUST_DIR/target/release/lib$PROJECT_NAME.dylib\"" > rust.gdextension
}

function create_new {
  PROJECT_NAME=$1
  PROJECT_SRC=$PROJECT_NAME/godot

  echo -e "\e[38;5;117mCreating project folder structure\e[0m"
  mkdir -p $PROJECT_SRC

  create_gdextension "$CURRENT_DIR/$PROJECT_NAME/godot" "$PROJECT_NAME" "../rust"
  create_rust_project "$CURRENT_DIR/$PROJECT_NAME" "$PROJECT_NAME"
}

function move_files_around {
  PROJECT_NAME=$1

  cd ..

  echo -e "\e[38;5;117m\nMoving files to a temporary folder\e[0m"
  mkdir ./.tmp-$PROJECT_NAME
  mv -v ./$PROJECT_NAME/* ./.tmp-$PROJECT_NAME/
  mv -v ./$PROJECT_NAME/.godot ./.tmp-$PROJECT_NAME/

  echo -e "\e[38;5;117m\nMoving the temporary folder files into the newly created folder\e[0m"
  mkdir -p $PROJECT_NAME/godot
  mv -v ./.tmp-$PROJECT_NAME/* $PROJECT_NAME/godot/
  mv -v ./.tmp-$PROJECT_NAME/.godot $PROJECT_NAME/godot/

  echo -e "\e[38;5;117m\nCleaning up the temporary folder\e[0m"
  rm -rf ./.tmp-$PROJECT_NAME
}

echo -e "\e[33mLooking for the \".godot\" folder in the current directory\e[0m"
if [ -d ".godot" ]; then
  PROJECT_NAME=${PWD##*/}
  echo -e "\e[32mFound \".godot\" folder\e[0m"
  read -p "Do you want to add (A) the project to the current directory or restructure (R) it? [a/r] " answer
  case ${answer:0:1} in
    # Add rust to the current directory in a subfolder
    a|A )
      echo "Adding project to the current directory"
      create_gdextension "$CURRENT_DIR" "$PROJECT_NAME" "rust"
      create_rust_project "$CURRENT_DIR/rust" "$PROJECT_NAME"
      echo -e "\e[32m\nDone!\e[0m"
    ;;
    # Restructure the project
    r|R )
      echo -e "\e[33mIt is recommended to backup your project with version control before proceeding.\e[0m"
      read -p "Are you sure you want to continue? [y/n] " proceed
      case ${proceed:0:1} in
        y|Y )
          echo -e "\e[38;5;117mRestructuring the project\e[0m"
          move_files_around "$PROJECT_NAME"
          create_gdextension "$CURRENT_DIR/godot" "$PROJECT_NAME" "../rust"
          create_rust_project  "$CURRENT_DIR" "$PROJECT_NAME"
          echo -e "\e[32m\nDone!\e[0m"
        ;;
        * )
          echo -e "\e[31mAborting\e[0m"
          exit 1
        ;;
      esac
    ;;
    * )
      echo -e "\e[31mInvalid input, please either enter \"a\" for add or \"r\" for restructure\e[0m"
    ;;
  esac
else
    echo -e "\e[31mNo \".godot\" folder found, lets create a new project\e[0m"
    read -p "Project name: " project_name
    create_new "$project_name"
    echo -e "\e[32m\nDone!\e[0m"
fi