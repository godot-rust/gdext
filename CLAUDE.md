# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project navigation

Always prefer ripgrep (`rg` command) to search inside files. 
Don't use `grep`, or weird `find` combinations, unless `rg` doesn't provide the needed functionality.

## Development Commands

Use the `check.sh` script for all development workflows:

```bash
# Run all checks (format, clippy, test, itest)
./check.sh

# Individual commands
./check.sh fmt          # Format code (fail if bad)
./check.sh clippy       # Lint with clippy
./check.sh test         # Unit tests (no Godot required)
./check.sh itest        # Integration tests (requires Godot)
./check.sh doc          # Generate docs
./check.sh dok          # Generate docs and open in browser

# Useful options
./check.sh --double                    # Test with double precision
./check.sh -f variant,static itest     # Filter integration tests
./check.sh -a 4.3 clippy               # Test against specific Godot version
```

Always run `./check.sh` before committing to ensure code quality.

## Architecture Overview

godot-rust is a multi-crate workspace providing Rust bindings for Godot 4.

- **`godot`** - Main public API crate (only crate users should depend on)
- **`godot-core`** - High-level Rust APIs (built-ins, classes, registration)
- **`godot-ffi`** - Low-level C bindings to Godot's GDExtension API
- **`godot-macros`** - Procedural macros (`#[derive(GodotClass)]`, `#[godot_api]`)
- **`godot-codegen`** - Code generation from Godot's API specification
- **`godot-bindings`** - Raw API bindings and build coordination
- **`godot-cell`** - Thread-safe memory management for Godot's single-threaded nature

Integration tests in `itest/` run Rust code within a Godot project.

Avoid the term "gdext" when possible, use "godot-rust" instead.

## Key Development Notes

### Testing Strategy
- Unit tests: `cargo test` (no Godot needed)
- Integration tests: Run within Godot engine via `./check.sh itest`
- Requires Godot 4.x binary (set `GODOT4_BIN` env var or have `godot4`/`godot` in PATH)

### API Versioning
- Supports Godot versions 4.1-4.4+ through feature flags (`api-4-1`, `api-4-2`, etc.)
- Use `--api-version` flag to test against specific versions
- When testing specific API versions, clippy is disabled by default due to potential conflicts
- Claude: just leave the default, it will test with latest Godot version.

### Important Cargo Features
- `experimental-threads` - Multi-threading support (experimental)
- `__codegen-full` - Generate entire Godot class API. Avoid this as it takes significantly longer and is rarely needed. Always ask for confirmation.

### Code Generation
Much of the API is auto-generated from Godot's specification. When working on generated code, check `godot-codegen/` for the generation logic.

### Contribution Guidelines
- One commit per logical change
- Do not change existing commits, add new ones. User can clean up.
- Avoid co-authoring as Claude.

### Code Style
- Comments must typically end in `.` (like proper sentences). Exception are short keywords.
- Import ordering follows rustfmt standard via `rustfmt.toml` configuration:
  - Standard library imports (`std`, `core`, `alloc`)
  - External crates (including workspace crates like `godot_ffi as sys`)
  - Local crate imports (`crate::`, `super::`)
  - Blank lines preserved between import groups

The `--no-default-features` flag is used by default in `check.sh` to reduce compile times.

### Documentation

- In RustDoc, API symbols like `MyClass` or `#[class(init)]` must be in backticks.

### Architecture Documents

- **[AsArg/ToGodot Integration Strategy](.claude/AsArgStrategy.md)** - Analysis and approaches for unifying conversion traits to reduce redundancy while preserving performance