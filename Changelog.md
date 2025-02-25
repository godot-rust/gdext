# Changelog

This document tracks changes of released library versions.

See also [Pulse](https://github.com/godot-rust/gdext/pulse/monthly) for recent activities.  
Cutting-edge API docs of the `master` branch are available [here](https://godot-rust.github.io/docs/gdext).

🌊 indicates a breaking change. Deprecations are not marked breaking.


## Quick navigation

- [v0.2.0](#v020), [v0.2.1](#v021), [v0.2.2](#v022), [v0.2.3](#v023), [v0.2.4](#v024)
- [v0.1.1](#v011), [v0.1.2](#v012), [v0.1.3](#v013)


## [v0.2.4](https://docs.rs/godot/0.2.4)

_24 February 2025_

### 🌻 Features

- Support additional doc comment markdown ([#1017](https://github.com/godot-rust/gdext/pull/1017))
- Provide safe way for implementing `IScriptExtension::instance_has` ([#1013](https://github.com/godot-rust/gdext/pull/1013))
- Global main thread ID ([#1045](https://github.com/godot-rust/gdext/pull/1045))
- Add `validate_property` virtual func ([#1030](https://github.com/godot-rust/gdext/pull/1030))

### 📈 Performance

- Speed up creating and extending packed arrays from iterators up to 63× ([#1023](https://github.com/godot-rust/gdext/pull/1023))

### 🧹 Quality of life

- Test generation of proc-macro code via declarative macro ([#1008](https://github.com/godot-rust/gdext/pull/1008))
- Test: support switching API versions in check.sh ([#1016](https://github.com/godot-rust/gdext/pull/1016))
- Support `Variant::object_id()` for Godot <4.4; add `object_id_unchecked()` ([#1034](https://github.com/godot-rust/gdext/pull/1034))
- Rename `Basis` + `Quaternion` methods, closer to Godot ([#1035](https://github.com/godot-rust/gdext/pull/1035))
- Compare scripts via object_id ([#1036](https://github.com/godot-rust/gdext/pull/1036))
- Add Godot 4.3 to minimal CI ([#1044](https://github.com/godot-rust/gdext/pull/1044))
- Add .uid files for GDScript + GDExtension resources; update script cache ([#1050](https://github.com/godot-rust/gdext/pull/1050))
- Add `#[allow(...)]` directives to macro-generated entries ([#1049](https://github.com/godot-rust/gdext/pull/1049))
- Clippy: disable excessive operator-precedence lint ([#1055](https://github.com/godot-rust/gdext/pull/1055))

### 🛠️ Bugfixes

- Fix `Variant` -> `Gd` conversions not taking into account dead objects ([#1033](https://github.com/godot-rust/gdext/pull/1033))
- Fix `DynGd<T,D>` export, implement proper export for `Array<DynGd<T,D>>` ([#1056](https://github.com/godot-rust/gdext/pull/1056))
- In `#[var]s`, handle renamed `#[func]`s ([#1019](https://github.com/godot-rust/gdext/pull/1019))
- 🌊 `#[signal]`: validate that there is no function body ([#1032](https://github.com/godot-rust/gdext/pull/1032))
- Disambiguate virtual method calls ([#1020](https://github.com/godot-rust/gdext/pull/1020))
- Parse doc strings with litrs ([#1015](https://github.com/godot-rust/gdext/pull/1015))
- Register-docs: don't duplicate brief in description ([#1053](https://github.com/godot-rust/gdext/pull/1053))
- Move some compile-time validations from godot to godot-ffi ([#1058](https://github.com/godot-rust/gdext/pull/1058))
- Clippy: fix "looks like a formatting argument but it is not part of a formatting macro" ([#1041](https://github.com/godot-rust/gdext/pull/1041))


## [v0.2.3](https://docs.rs/godot/0.2.3)

_30 January 2025_

### 🌻 Features

- Map `Vector2.Axis` and `Vector2i.Axis` to `Vector2Axis` enum ([#1012](https://github.com/godot-rust/gdext/pull/1012))
- Implement `Var` and `Export` for `DynGd<T, D>` ([#998](https://github.com/godot-rust/gdext/pull/998))
- Support associated types in `#[godot_dyn]` ([#1022](https://github.com/godot-rust/gdext/pull/1022))
- Add `Aabb::intersect_ray()` ([#1001](https://github.com/godot-rust/gdext/pull/1001))
- FFI: compatibility layer for virtual methods ([#991](https://github.com/godot-rust/gdext/pull/991), [#1007](https://github.com/godot-rust/gdext/pull/1007))
- FFI: postinit create, icon paths ([#991](https://github.com/godot-rust/gdext/pull/991))

### 🧹 Quality of life

- Follow clippy 1.84; limit `NodePath::subpath()` polyfill again ([#1010](https://github.com/godot-rust/gdext/pull/1010))
- Remove dead binding code regarding Godot 4.0 ([#1014](https://github.com/godot-rust/gdext/pull/1014))
- API consistency for bounding boxes ([#1001](https://github.com/godot-rust/gdext/pull/1001))
- Document and refactor `PluginItem` related stuff ([#1003](https://github.com/godot-rust/gdext/pull/1003))

### 🛠️ Bugfixes

- Fix nightly compiler warnings about `#[cfg(before_api = "4.3")]` in the generated `#[godot_api]` impl ([#995](https://github.com/godot-rust/gdext/pull/995))
- Fix `#[derive(Var)]` generating incorrect `hint_string` for enums ([#1011](https://github.com/godot-rust/gdext/pull/1011))

### 📚 Documentation

- Document + test limitations of `Callable::from_local_static()` ([#1004](https://github.com/godot-rust/gdext/pull/1004))
- Document builtin API design ([#999](https://github.com/godot-rust/gdext/pull/999))


## [v0.2.2](https://docs.rs/godot/0.2.2)

_31 December 2024_

### 🌻 Features

- Feature parity with Godot builtin types
  - `Vector2i` ([#978](https://github.com/godot-rust/gdext/pull/978))
  - `Projection` ([#983](https://github.com/godot-rust/gdext/pull/983))
  - `Callable` ([#979](https://github.com/godot-rust/gdext/pull/979))
  - `Quaternion` ([#981](https://github.com/godot-rust/gdext/pull/981))
  - `GString` + `StringName` ([#980](https://github.com/godot-rust/gdext/pull/980))
  - `NodePath` ([#982](https://github.com/godot-rust/gdext/pull/982))
  - `PackedByteArray` ([#994](https://github.com/godot-rust/gdext/pull/994))
- Support static functions in `Callable` ([#989](https://github.com/godot-rust/gdext/pull/989))
- Codegen can directly expose `Inner*` builtin methods ([#976](https://github.com/godot-rust/gdext/pull/976))
- Generate builtin methods with varargs ([#977](https://github.com/godot-rust/gdext/pull/977))

### 🧹 Quality of life

- More accurately provide spans to errors in the `GodotClass` macro ([#920](https://github.com/godot-rust/gdext/pull/920))
- Improve some proc-macro attribute error messages ([#971](https://github.com/godot-rust/gdext/pull/971))
- Add required virtual method `IScriptInstance::get_doc_class_name()` in test ([#975](https://github.com/godot-rust/gdext/pull/975))
- Clean up `Callable` + tests, fix `check.sh test` ([#990](https://github.com/godot-rust/gdext/pull/990))

### 📚 Documentation

- Improve docs in `DynGd` (re-enrichment) + Cargo features ([#969](https://github.com/godot-rust/gdext/pull/969))


## [v0.2.1](https://docs.rs/godot/0.2.1)

_8 December 2024_

### 🌻 Features

- `#[godot_api(secondary)]` for multiple impl blocks ([#927](https://github.com/godot-rust/gdext/pull/927))
- `DynGd<T, D>` smart pointer for Rust-side dynamic dispatch ([#953](https://github.com/godot-rust/gdext/pull/953), [#958](https://github.com/godot-rust/gdext/pull/958))
- `Callable::from_local_fn()` + `Callable::from_sync_fn()` ([#965](https://github.com/godot-rust/gdext/pull/965))
- Add `Variant::object_id()` ([#914](https://github.com/godot-rust/gdext/pull/914))

### 🧹 Quality of life

- `#[gdextension]` macro: rename `entry_point` -> `entry_symbol` and write docs ([#959](https://github.com/godot-rust/gdext/pull/959))
- Use `GDExtensionCallableCustomInfo2` instead of deprecated `GDExtensionCallableCustomInfo` ([#952](https://github.com/godot-rust/gdext/pull/952))
- `sys::short_type_name`, conversions and relaxed `GodotType` ([#957](https://github.com/godot-rust/gdext/pull/957))
- Clippy (elided lifetimes) + rustfmt ([#956](https://github.com/godot-rust/gdext/pull/956))
- Add test verifying that custom callables don't crash when `Err` is returned ([#950](https://github.com/godot-rust/gdext/pull/950))
- Minor signal cleanup; prevent `#[signal]` from being used in secondary impl blocks ([#964](https://github.com/godot-rust/gdext/pull/964))

### 🛠️ Bugfixes

- Fix `#[godot_dyn]` causing error when implemented for two traits ([#962](https://github.com/godot-rust/gdext/pull/962))
- Prevent abort on double-panic if single-threading check fails ([#965](https://github.com/godot-rust/gdext/pull/965))

### 📚 Documentation

- `#[gdextension]` macro: rename `entry_point` -> `entry_symbol` and write docs ([#959](https://github.com/godot-rust/gdext/pull/959))
- Helpful doc aliases: `func`, `var`, `init`, .. ([#960](https://github.com/godot-rust/gdext/pull/960))


## [v0.2.0](https://docs.rs/godot/0.2.0)

_15 November 2024_

See [devlog article](https://godot-rust.github.io/dev/november-2024-update) for highlights.

### 🌻 Features

- Godot 4.3 support in CI and `api-4-3` feature ([#859](https://github.com/godot-rust/gdext/pull/859))
- 🌊 Drop support for Godot 4.0 ([#820](https://github.com/godot-rust/gdext/pull/820))
- 🌊 Ergonomic arguments
  - `AsObjectArg` trait enabling implicit conversions for object parameters ([#800](https://github.com/godot-rust/gdext/pull/800))
  - Pass-by-ref for non-`Copy` builtins ([#900](https://github.com/godot-rust/gdext/pull/900), [#906](https://github.com/godot-rust/gdext/pull/906))
  - String argument conversion + `AsArg` trait ([#940](https://github.com/godot-rust/gdext/pull/940))
  - `Callable` is now passed by-ref ([#944](https://github.com/godot-rust/gdext/pull/944))
  - Require `AsObjectArg` pass-by-ref, consistent with `AsArg` ([#847](https://github.com/godot-rust/gdext/pull/847), [#947](https://github.com/godot-rust/gdext/pull/947))
- Godot docs from RustDoc comments
  - Generate documentation from doc comments ([#748](https://github.com/godot-rust/gdext/pull/748))
  - Generate valid XML from doc comments ([#861](https://github.com/godot-rust/gdext/pull/861))
  - Include `register-docs` feature in CI ([#819](https://github.com/godot-rust/gdext/pull/819))
- RPC attributes
  - Add `#[rpc]` attribute to user-defined functions ([#902](https://github.com/godot-rust/gdext/pull/902))
- Registration APIs
  - 🌊 Add `OnReady::node()` + `#[init(node = "...")]` attribute ([#807](https://github.com/godot-rust/gdext/pull/807))
  - Support Unicode class names (Godot 4.4+) ([#891](https://github.com/godot-rust/gdext/pull/891))
  - Unicode support in `ClassName::new_cached()`; adjust test ([#899](https://github.com/godot-rust/gdext/pull/899))
  - Derive `Ord` and `PartialOrd` for `ClassName` ([#928](https://github.com/godot-rust/gdext/pull/928))
  - Implement `Debug` for `InitState` and `OnReady` ([#879](https://github.com/godot-rust/gdext/pull/879))
- Enums
  - `#[derive(GodotClass)]` enums can now have complex ordinal expressions ([#843](https://github.com/godot-rust/gdext/pull/843))
  - Enums can now be bit-combined with known masks ([#857](https://github.com/godot-rust/gdext/pull/857))
  - Add `as_str` and `godot_name` to non-bitfield enums ([#898](https://github.com/godot-rust/gdext/pull/898))
- Required virtual functions
  - Detect whether virtual functions are required to override ([#904](https://github.com/godot-rust/gdext/pull/904))
  - 🌊 Required virtual methods should be required at compile-time ([#771](https://github.com/godot-rust/gdext/pull/771))
- Conversions + operators
  - Implement `GodotConvert` for `Vec<T>`, `[T; N]` and `&[T]` ([#795](https://github.com/godot-rust/gdext/pull/795))
  - Implement `From<&[char]>` for `GString` ([#862](https://github.com/godot-rust/gdext/pull/862))
  - Add `From<[Elem; N]>` for Packed Array and Optimize `From<Vec<Elem>>` ([#827](https://github.com/godot-rust/gdext/pull/827))
  - Handle typed array metadata ([#855](https://github.com/godot-rust/gdext/pull/855))
  - Vector conversion functions ([#824](https://github.com/godot-rust/gdext/pull/824))
  - Add Mul operator for Quaternion + Vector3 ([#894](https://github.com/godot-rust/gdext/pull/894))

### 📈 Performance

- `RawGd`: cache pointer to internal storage ([#831](https://github.com/godot-rust/gdext/pull/831))
- `ClassName` now dynamic and faster ([#834](https://github.com/godot-rust/gdext/pull/834))
- Pass-by-ref for non-`Copy` builtins (backend) ([#906](https://github.com/godot-rust/gdext/pull/906))

### 🧹 Quality of life

- Renames and removals
  - 🌊 Remove deprecated symbols from before v0.1 ([#808](https://github.com/godot-rust/gdext/pull/808))
  - Deprecate instance utilities in `godot::global` ([#901](https://github.com/godot-rust/gdext/pull/901))
  - Shorten `#[init(default = ...)]` to `#[init(val = ...)]` ([#844](https://github.com/godot-rust/gdext/pull/844))
  - `#[class]` attribute: rename `hidden` -> `internal`, deprecate `editor_plugin` ([#884](https://github.com/godot-rust/gdext/pull/884))
  - Cleanup around `godot::meta` argument conversions ([#948](https://github.com/godot-rust/gdext/pull/948))
  - Remove `to_2d()` + `to_3d()`; clean up `ApiParam` ([#943](https://github.com/godot-rust/gdext/pull/943))
  - 🌊 Simplify property hint APIs ([#838](https://github.com/godot-rust/gdext/pull/838))
- Validation
  - Fix validation for `api-*` mutual exclusivity ([#809](https://github.com/godot-rust/gdext/pull/809))
  - Validate that virtual extension classes require `#[class(tool)]` ([#850](https://github.com/godot-rust/gdext/pull/850))
  - Validate that editor plugin classes require `#[class(tool)]` ([#852](https://github.com/godot-rust/gdext/pull/852))
  - Best-effort checks for `Array<Integer>` conversions; fix `Debug` for variants containing typed arrays ([#853](https://github.com/godot-rust/gdext/pull/853))
  - 🌊 Disallow `Export` if class doesn't inherit `Node` or `Resource` ([#839](https://github.com/godot-rust/gdext/pull/839))
  - 🌊 Validate that Nodes can only be exported from Node-derived classes ([#841](https://github.com/godot-rust/gdext/pull/841))
- CI and tooling
  - Cargo-deny maintenance: update to advisories/licenses v2 ([#829](https://github.com/godot-rust/gdext/pull/829))
  - CI runner updates ([#941](https://github.com/godot-rust/gdext/pull/941))
  - Skip `notify-docs` job when running in a fork ([#945](https://github.com/godot-rust/gdext/pull/945))
  - Allow manually triggering `full-ci` workflow (mostly useful for forks) ([#933](https://github.com/godot-rust/gdext/pull/933))
- Code generation and Godot APIs
  - Allow codegen for `UniformSetCacheRD` for Godot >=4.3 ([#816](https://github.com/godot-rust/gdext/pull/816))
  - Enable `ResourceLoader::load_threaded_*` with `experimental-threads` ([#856](https://github.com/godot-rust/gdext/pull/856))
  - Dependency update, more tests for vector angle functions ([#860](https://github.com/godot-rust/gdext/pull/860))
- Upstream follow-up
  - 🌊 Support `GDExtensionScriptInstanceInfo3` in 4.3 ([#849](https://github.com/godot-rust/gdext/pull/849))
  - Support meta `char16` and `char32` ([#895](https://github.com/godot-rust/gdext/pull/895))
  - Add `GodotConvert` impl for `*const u8` pointers ([#866](https://github.com/godot-rust/gdext/pull/866))
  - Update list of experimental classes ([#897](https://github.com/godot-rust/gdext/pull/897))
  - Update hint_string tests to account for Godot 4.4 floats with `.0` formatting ([#936](https://github.com/godot-rust/gdext/pull/936))
- Panics
  - Disable panic hooks in Release mode ([#889](https://github.com/godot-rust/gdext/pull/889))
  - In debug, include location information in error message on panic ([#926](https://github.com/godot-rust/gdext/pull/926))
- Refactoring
  - Rewrite `#[var]` + `#[export]` registration to use type-safe API behind scenes ([#840](https://github.com/godot-rust/gdext/pull/840))
  - Get rid of placeholder names like "foo" ([#888](https://github.com/godot-rust/gdext/pull/888))

### 🛠️ Bugfixes

- Argument passing
  - Set null into Godot Engint APIs nullable parameters as default ([#823](https://github.com/godot-rust/gdext/pull/823))
  - Fix `Ex*` builder parameters: `ObjectArg<T>` -> `impl AsObjectArg<T>` ([#830](https://github.com/godot-rust/gdext/pull/830))
- Godot doc generation from RustDoc
  - Fix doc comments not showing up if only some class members are documented ([#815](https://github.com/godot-rust/gdext/pull/815))
  - Fix `register-docs` feature not being tested ([#942](https://github.com/godot-rust/gdext/pull/942))
- Registration
  - Fix `Array<T>` registered without element type ([#836](https://github.com/godot-rust/gdext/pull/836))
  - Virtual methods now take `Option<Gd<T>>` (unless whitelisted) ([#883](https://github.com/godot-rust/gdext/pull/883))
  - Make arrays exportable only when their inner type is exportable ([#875](https://github.com/godot-rust/gdext/pull/875))
  - Display script-virtual methods as `_method` instead of `method` in Godot docs ([#918](https://github.com/godot-rust/gdext/pull/918))
  - Implement the `safe_ident` strategy for virtual call parameter identifier generation ([#822](https://github.com/godot-rust/gdext/pull/822))
- FFI and memory safety
  - Fix user-after-free in `AsObjectArg` pass-by-value (in default-param methods) ([#846](https://github.com/godot-rust/gdext/pull/846))
  - `RawGd::move_return_ptr` with `PtrcallType::Virtual` leaks reference ([#848](https://github.com/godot-rust/gdext/pull/848))
  - Don't abort on panic inside Callable ([#873](https://github.com/godot-rust/gdext/pull/873))
- Tooling and dependencies
  - Dev-dependencies are enabling full codegen ([#842](https://github.com/godot-rust/gdext/pull/842))
  - OpenXR is not available on Web ([#872](https://github.com/godot-rust/gdext/pull/872))
  - Fix `enum_test.rs` accidentally excluded from itest ([#931](https://github.com/godot-rust/gdext/pull/931))
  - Codegen-rustfmt: use 2021 edition ([#937](https://github.com/godot-rust/gdext/pull/937))
- Math
  - `Vecor3::sign()` gives incorrect results due to `i32` conversion ([#865](https://github.com/godot-rust/gdext/pull/865))

### 📚 Documentation

- Builtin docs (impl blocks, navigation table, link to Godot) ([#821](https://github.com/godot-rust/gdext/pull/821))
- Add docs for `#[rpc]` ([#949](https://github.com/godot-rust/gdext/pull/949))
- Overview about type conversions ([#833](https://github.com/godot-rust/gdext/pull/833))
- Document `godot::meta` argument conversions ([#948](https://github.com/godot-rust/gdext/pull/948))
- Add a doc to point users to kwarg builders ([#876](https://github.com/godot-rust/gdext/pull/876))
- Resolve doc warning with global enums ([#896](https://github.com/godot-rust/gdext/pull/896))
- ReadMe update + clippy error ([#929](https://github.com/godot-rust/gdext/pull/929))


## [v0.1.3](https://docs.rs/godot/0.1.3)

_22 July 2024_

### 🧹 Quality of life

- Add helpful error for renamed Wasm module ([#799](https://github.com/godot-rust/gdext/pull/799))
- More thoroughly document `unsafe` in `godot-ffi` ([#774](https://github.com/godot-rust/gdext/pull/774))

### 🛠️ Bugfixes

- Map `Vector3i.Axis` enum to builtin `Vector3Axis` ([#797](https://github.com/godot-rust/gdext/pull/797))
- Prevent `out!` from actually formatting the input if disabled ([#801](https://github.com/godot-rust/gdext/pull/801))
- Disable `main_thread_id` assertion for Android debug build ([#780](https://github.com/godot-rust/gdext/pull/780))
- `GdCell::borrow_mut` should block on main thread if shared ref exists ([#787](https://github.com/godot-rust/gdext/pull/787))

### 📚 Documentation

- Typos + code reordering ([#802](https://github.com/godot-rust/gdext/pull/802))
- Add crates.io ReadMe + docs logo ([#804](https://github.com/godot-rust/gdext/pull/804))


## [v0.1.2](https://docs.rs/godot/0.1.2)

_15 July 2024_

### 🌻 Features

- Add more `normalized` functions ([#761](https://github.com/godot-rust/gdext/pull/761))
- Add conversion from `Vec<$Element>` to `$PackedArray` types ([#785](https://github.com/godot-rust/gdext/pull/785))
- Add `snapped` to integer vectors ([#768](https://github.com/godot-rust/gdext/pull/768))
- Add determinant to `Transform2D` ([#770](https://github.com/godot-rust/gdext/pull/770))
- Support `#[export(range = (radians_as_degrees, suffix=XX))]` ([#783](https://github.com/godot-rust/gdext/pull/783))
- Add support for `nothreads` Wasm builds (Godot 4.3+) ([#794](https://github.com/godot-rust/gdext/pull/794))

### 🧹 Quality of life

- Reorder compile errors for `#[derive(GodotClass)]` ([#773](https://github.com/godot-rust/gdext/pull/773))
- Change `Global` to use `Once` ([#752](https://github.com/godot-rust/gdext/pull/752))
- Prevent global `CallError` tracker from growing indefinitely ([#798](https://github.com/godot-rust/gdext/pull/798))

### 🛠️ Bugfixes

- Change logic to disable `OpenXR` for iOS ([#781](https://github.com/godot-rust/gdext/pull/781))
- Pointer is already `*const u32` on aarch64 ([#788](https://github.com/godot-rust/gdext/pull/788))
- Handle panics in virtual interface methods ([#757](https://github.com/godot-rust/gdext/pull/757))

### 📚 Documentation

- Document why `Basis` columns are `a`, `b`, and `c` ([#776](https://github.com/godot-rust/gdext/pull/776))


## [v0.1.1](https://docs.rs/godot/0.1.1)

_24 June 2024_

Initial release on crates.io. See [devlog article](https://godot-rust.github.io/dev/june-2024-update).
