# Changelog

This document tracks changes of released library versions.

See also [Pulse](https://github.com/godot-rust/gdext/pulse/monthly) for recent activities.  
Cutting-edge API docs of the `master` branch are available [here](https://godot-rust.github.io/docs/gdext).

🌊 indicates a breaking change. Deprecations are not marked breaking.


## Quick navigation

- [v0.4.0](#v040), [v0.4.1](#v041), [v0.4.2](#v042), [v0.4.3](#v043), [v0.4.4](#v044), [v0.4.5](#v045)
- [v0.3.0](#v030), [v0.3.1](#v031), [v0.3.2](#v032), [v0.3.3](#v033), [v0.3.4](#v034), [v0.3.5](#v035)
- [v0.2.0](#v020), [v0.2.1](#v021), [v0.2.2](#v022), [v0.2.3](#v023), [v0.2.4](#v024)
- [v0.1.1](#v011), [v0.1.2](#v012), [v0.1.3](#v013)


## [v0.4.5](https://docs.rs/godot/0.4.5)

_12 December 2025_

Hotfix release: backports an update due to a breaking change in the Rust compiler.

### 🛠️ Bugfixes

- Fix compiler error caused by raw pointer cast no longer extending lifetimes ([#1441](https://github.com/godot-rust/gdext/pull/1441))


## [v0.4.4](https://docs.rs/godot/0.4.4)

_4 December 2025_

### 🌻 Features

- Fix `Vector2i` using FFI; add functions to `Vector3i`/`Vector4i` ([#1418](https://github.com/godot-rust/gdext/pull/1418))
- Allow to register user-defined engine singletons ([#1399](https://github.com/godot-rust/gdext/pull/1399))
- Add `StringName::chars()` ([#1419](https://github.com/godot-rust/gdext/pull/1419))

### 📈 Performance

- Fix `Vector2i` using FFI; add functions to `Vector3i`/`Vector4i` ([#1418](https://github.com/godot-rust/gdext/pull/1418))

### 🧹 Quality of life

- Rename untyped collections to `VarArray` + `VarDictionary` ([#1428](https://github.com/godot-rust/gdext/pull/1428))
- Run integration tests in `-e --headless` mode ([#1432](https://github.com/godot-rust/gdext/pull/1432))

### 🛠️ Bugfixes

- Add special casing for `PrimitiveMesh` methods ([#1430](https://github.com/godot-rust/gdext/pull/1430))

### 📚 Documentation

- Update comment to use `to_cow_str()` for `class_id()` ([#1412](https://github.com/godot-rust/gdext/pull/1412))


## [v0.4.3](https://docs.rs/godot/0.4.3)

_26 November 2025_

### 🌻 Features

- Safeguard levels: fine-tune the amount of runtime validations ([#1278](https://github.com/godot-rust/gdext/pull/1278))
- Support `rename` for `#[var]` ([#1388](https://github.com/godot-rust/gdext/pull/1388))
- Add `Array::functional_ops()` ([#1393](https://github.com/godot-rust/gdext/pull/1393))
- Default parameters via `#[opt(default = ...)]` syntax ([#1396](https://github.com/godot-rust/gdext/pull/1396))

### 📈 Performance

- Only provide function name to `CallContext` in debug ([#1331](https://github.com/godot-rust/gdext/pull/1331))

### 🧹 Quality of life

- Failed ptrcalls: instead of panic, print error + return default ([#1387](https://github.com/godot-rust/gdext/pull/1387))
- `#[bench(manual)]` for more control, including setup ([#1390](https://github.com/godot-rust/gdext/pull/1390))
- Add `GodotImmutable` to constrain `#[opt(default)]` types ([#1406](https://github.com/godot-rust/gdext/pull/1406))
- Update `runtime_version` to use non-deprecated `get_godot_version` pointer ([#1394](https://github.com/godot-rust/gdext/pull/1394))
- Start phasing out `#[cfg(debug_assertions)]` in favor of safeguards ([#1395](https://github.com/godot-rust/gdext/pull/1395))
- Remove macOS x86 (Intel) from CI ([#1402](https://github.com/godot-rust/gdext/pull/1402))
- Remove unnecessary `.ord()` calls for enums/bitfields ([#1414](https://github.com/godot-rust/gdext/pull/1414))
- Remove `experimental-required-objs` Cargo feature ([#1416](https://github.com/godot-rust/gdext/pull/1416))
- Various small cleanups ([#1386](https://github.com/godot-rust/gdext/pull/1386))

### 🛠️ Bugfixes

- Fix non-deterministic `register-docs` XML output ([#1391](https://github.com/godot-rust/gdext/pull/1391))
- `__are_oneditor_fields_initalized` needs explicit usage of Singleton trait ([#1400](https://github.com/godot-rust/gdext/pull/1400))
- Blocklist `max_align_t` from bindings generation ([#1401](https://github.com/godot-rust/gdext/pull/1401))
- Prevent procedural macro hygiene issue with eager macros ([#1397](https://github.com/godot-rust/gdext/pull/1397))
- Fix `get_godot_version2` unavailable on older builds ([#1403](https://github.com/godot-rust/gdext/pull/1403))
- Temporarily disable `GString::find()` itest due to upstream bug ([#1411](https://github.com/godot-rust/gdext/pull/1411))


## [v0.4.2](https://docs.rs/godot/0.4.2)

_26 October 2025_

### 🌻 Features

- Simple API to fetch autoloads ([#1381](https://github.com/godot-rust/gdext/pull/1381))
- Experimental support for required parameters/returns in Godot APIs ([#1383](https://github.com/godot-rust/gdext/pull/1383))

### 🧹 Quality of life

- `ExtensionLibrary::on_main_loop_*`: merge into new `on_stage_init/deinit` API ([#1380](https://github.com/godot-rust/gdext/pull/1380))
- Rename builtin `hash()` -> `hash_u32()`; add tests ([#1366](https://github.com/godot-rust/gdext/pull/1366))

### 🛠️ Bugfixes

- Backport Godot fix for incorrect `Glyph` native-struct ([#1369](https://github.com/godot-rust/gdext/pull/1369))
- Validate call params for `gd_self` virtual methods ([#1382](https://github.com/godot-rust/gdext/pull/1382))
- Fix codegen regression: `Array<Option<Gd>>` -> `Array<Gd>` ([#1385](https://github.com/godot-rust/gdext/pull/1385))


## [v0.4.1](https://docs.rs/godot/0.4.1)

_23 October 2025_

### 🌻 Features

- Add main loop callbacks to `ExtensionLibrary` ([#1313](https://github.com/godot-rust/gdext/pull/1313), [#1380](https://github.com/godot-rust/gdext/pull/1380))
- Class Docs – register docs in `#[godot_api(secondary)]`, simplify docs registration logic ([#1355](https://github.com/godot-rust/gdext/pull/1355))
- Codegen: support sys types in engine APIs ([#1363](https://github.com/godot-rust/gdext/pull/1363), [#1365](https://github.com/godot-rust/gdext/pull/1365))

### 📈 Performance

- Use Rust `str` instead of `CStr` in `ClassIdSource` ([#1334](https://github.com/godot-rust/gdext/pull/1334))

### 🧹 Quality of life

- Preserve doc comments for signal ([#1353](https://github.com/godot-rust/gdext/pull/1353))
- Provide error context for typed array clone check ([#1348](https://github.com/godot-rust/gdext/pull/1348))
- Improve spans; use tuple type for virtual signatures ([#1370](https://github.com/godot-rust/gdext/pull/1370))
- Preserve span of arguments for better compile errors ([#1373](https://github.com/godot-rust/gdext/pull/1373))
- Update to litrs 1.0 ([#1377](https://github.com/godot-rust/gdext/pull/1377))
- Allow opening itests in editor ([#1379](https://github.com/godot-rust/gdext/pull/1379))

### 🛠️ Bugfixes

- Ease `AsArg<Option<Gd<T>>>` bounds to make it usable with signals ([#1371](https://github.com/godot-rust/gdext/pull/1371))
- Handle panic in OnReady `auto_init` ([#1351](https://github.com/godot-rust/gdext/pull/1351))
- Update `GFile::read_as_gstring_entire()` after Godot removes `skip_cr` parameter ([#1349](https://github.com/godot-rust/gdext/pull/1349))
- Fix `Callable::from_sync_fn` doc example using deprecated `Result<T>` return ([#1347](https://github.com/godot-rust/gdext/pull/1347))
- Deprecate `#[class(no_init)]` for editor plugins ([#1378](https://github.com/godot-rust/gdext/pull/1378))
- Initialize and cache proper return value for generic, typed array ([#1357](https://github.com/godot-rust/gdext/pull/1357))
- Fix hot-reload crashes on macOS when the `.gdextension` file changes ([#1367](https://github.com/godot-rust/gdext/pull/1367))


## [v0.4.0](https://docs.rs/godot/0.4.0)

_29 September 2025_

### 🌻 Features

- Godot 4.5 API level ([#1339](https://github.com/godot-rust/gdext/pull/1339))
- Allow to use `#[func(gd_self)]` with Interface methods ([#1282](https://github.com/godot-rust/gdext/pull/1282))
- Generic `PackedArray<T>` ([#1291](https://github.com/godot-rust/gdext/pull/1291))
- 🌊 Numeric `#[export]` limits and type checks ([#1320](https://github.com/godot-rust/gdext/pull/1320))
- 🌊 Argument passing: new `ToGodot::Pass` is either `ByValue`/`ByRef` ([#1285](https://github.com/godot-rust/gdext/pull/1285))
- 🌊 More type-safe engine APIs (mostly int -> enum) ([#1315](https://github.com/godot-rust/gdext/pull/1315))
- Emit `POSTINITIALIZE` notification after `init()` ([#1211](https://github.com/godot-rust/gdext/pull/1211))
- Add `TypedSignal::to_untyped()` ([#1288](https://github.com/godot-rust/gdext/pull/1288))
- Add `Dictionary::get_or_insert()` ([#1295](https://github.com/godot-rust/gdext/pull/1295))
- Add `ElementType`; expose it in arrays and dictionaries ([#1304](https://github.com/godot-rust/gdext/pull/1304))
- Migrate inherent `singleton()` fn to new `Singleton` trait ([#1325](https://github.com/godot-rust/gdext/pull/1325))

### 📈 Performance

- `base()` + `base_mut()` no longer clone `Gd` pointer ([#1302](https://github.com/godot-rust/gdext/pull/1302))
- Restore `ToGodot` pass-by-ref for objects ([#1310](https://github.com/godot-rust/gdext/pull/1310))
- Remove static lifetime in `StringName::from(&CStr)` ([#1307](https://github.com/godot-rust/gdext/pull/1307))
- `AsArg` for objects now consistently passes by reference ([#1314](https://github.com/godot-rust/gdext/pull/1314))
- 🌊 Argument passing: new `ToGodot::Pass` is either `ByValue`/`ByRef` ([#1285](https://github.com/godot-rust/gdext/pull/1285))
- 🌊 Remove by-value `From` conversions between strings ([#1286](https://github.com/godot-rust/gdext/pull/1286))

### 🧹 Quality of life

- 🌊 Remove support for Godot 4.1 ([#1292](https://github.com/godot-rust/gdext/pull/1292))
- `MultiplayerApi`: getters now use `&self` receiver rather than `&mut self` ([#1274](https://github.com/godot-rust/gdext/pull/1274))
- `ClassName` construction from dynamic values ([#1298](https://github.com/godot-rust/gdext/pull/1298))
- Implement Send and Sync for PhantomVar ([#1305](https://github.com/godot-rust/gdext/pull/1305))
- Add `ClassCodegenLevel::Core` ([#1289](https://github.com/godot-rust/gdext/pull/1289))
- 🌊 Merge `AsObjectArg<T>` into `AsArg<Gd<T>>` ([#1308](https://github.com/godot-rust/gdext/pull/1308))
- 🌊 Add `SignedRange` for negative indices in range ops ([#1300](https://github.com/godot-rust/gdext/pull/1300))
- 🌊 Require that classes starting with "Editor" be marked internal ([#1272](https://github.com/godot-rust/gdext/pull/1272))
- Address `clippy::nursery` lint errors in macros ([#1317](https://github.com/godot-rust/gdext/pull/1317))
- FFI: make `AsArg` internals safer to use ([#1321](https://github.com/godot-rust/gdext/pull/1321))
- `AsArg` now supports `Option<DynGd>` ([#1323](https://github.com/godot-rust/gdext/pull/1323))
- Rename `ClassName` -> `ClassId` ([#1322](https://github.com/godot-rust/gdext/pull/1322))
- 🌊 Split `apply_deferred()` -> `run_deferred()` + `run_deferred_gd()` ([#1327](https://github.com/godot-rust/gdext/pull/1327))
- 🌊 `StringName`: remove `From` impl for `&'static CStr` ([#1316](https://github.com/godot-rust/gdext/pull/1316))
- 🌊 `Callable::from_local_fn()` now returns `R: ToGodot` ([#1332](https://github.com/godot-rust/gdext/pull/1332))
- 🌊 Add higher-level `CallErrorType` for script instance APIs ([#1333](https://github.com/godot-rust/gdext/pull/1333))
- 🌊 Remove deprecated symbols, other small cleanups ([#1340](https://github.com/godot-rust/gdext/pull/1340))
- 🌊 `Callable::from_linked_fn` now returns `R: ToGodot` instead of Variant ([#1344](https://github.com/godot-rust/gdext/pull/1344))
- `Callable`: rename `from_local_fn` -> `from_fn`, keep deprecated `from_local_fn` with old signature ([#1346](https://github.com/godot-rust/gdext/pull/1346))

### 🛠️ Bugfixes

- Fix `ThreadConfined<T>` using thread-unsafe `Drop` ([#1283](https://github.com/godot-rust/gdext/pull/1283))
- Emit proper compile error when `#[rpc]` attribute is used in `#[godot_api(secondary)]` block ([#1294](https://github.com/godot-rust/gdext/pull/1294))
- Drop_strong_ref – make sure that given instance haven't been freed before dropping stored StrongRef ([#1301](https://github.com/godot-rust/gdext/pull/1301))
- Improve init-level tests; fix `RenderingServer` singleton error ([#1309](https://github.com/godot-rust/gdext/pull/1309))
- Remove lifetime from `PassiveGd` ([#1312](https://github.com/godot-rust/gdext/pull/1312))
- Codegen excludes classes only on certain targets ([#1338](https://github.com/godot-rust/gdext/pull/1338))
- Fix typo in `std::mem::transmute` causing UB while using `AsArg<DynGd<Base, D>>` ([#1345](https://github.com/godot-rust/gdext/pull/1345))
- Itest: work around clippy errors when `quote!` generates one big line ([#1336](https://github.com/godot-rust/gdext/pull/1336))

### 📚 Documentation

- ReadMe: update + code example ([#1342](https://github.com/godot-rust/gdext/pull/1342))


## [v0.3.5](https://docs.rs/godot/0.3.5)

_18 August 2025_

### 🌻 Features

- Implement `Gd::try_dynify` ([#1255](https://github.com/godot-rust/gdext/pull/1255))
- Add `PhantomVar<T>` to support properties without a backing field ([#1261](https://github.com/godot-rust/gdext/pull/1261))
- Access to base pointer during initialization ([#1273](https://github.com/godot-rust/gdext/pull/1273))

### 🧹 Quality of life

- `match_class!` fallback branch is now optional for `()` ([#1246](https://github.com/godot-rust/gdext/pull/1246))
- `match_class!` now supports `_ @ Class` discard pattern ([#1252](https://github.com/godot-rust/gdext/pull/1252))
- Add bounds for future-proof `Gd` deref ([#1254](https://github.com/godot-rust/gdext/pull/1254))
- Mark `GString` as `Send` ([#1260](https://github.com/godot-rust/gdext/pull/1260))
- Properly register GDExtension `reference`/`unreference` callbacks ([#1270](https://github.com/godot-rust/gdext/pull/1270))
- Code style: stricter imports ([#1269](https://github.com/godot-rust/gdext/pull/1269))
- Nightly rustfmt + internals (once-calls, robust refcounts) ([#1271](https://github.com/godot-rust/gdext/pull/1271))

### 🛠️ Bugfixes

- Fix `is_main_thread` being gated behind `#[cfg(not(wasm_nothreads))]` despite being necessary to build wasm nothread ([#1251](https://github.com/godot-rust/gdext/pull/1251))
- Godot can use a different "main thread" for the main loop ([#1253](https://github.com/godot-rust/gdext/pull/1253))
- Fix `node_call_group` test accidentally renaming root tree ([#1277](https://github.com/godot-rust/gdext/pull/1277))

### 📚 Documentation

- Add extra class docs for `ResourceFormatLoader` to mention that the `experimental-threads` feature is required ([#1258](https://github.com/godot-rust/gdext/pull/1258))
- Clarify + test `Packed*Array` behavior w.r.t. copy-and-write + `#[var]` ([#1268](https://github.com/godot-rust/gdext/pull/1268))


## [v0.3.4](https://docs.rs/godot/0.3.4)

_22 July 2025_

### 🧹 Quality of life

- Derive `Hash` for `Rect2i` and a few enums ([#1241](https://github.com/godot-rust/gdext/pull/1241))
- Emit proper compile error while trying to `#[export]` `Gd<T>` or `DynGd<T, D>` ([#1243](https://github.com/godot-rust/gdext/pull/1243))

### 🛠️ Bugfixes

- Re-export deprecated `dict!` macro ([#1247](https://github.com/godot-rust/gdext/pull/1247))


## [v0.3.3](https://docs.rs/godot/0.3.3)

_21 July 2025_

### 🌻 Features

- `match_class!` macro to dispatch subclasses ([#1225](https://github.com/godot-rust/gdext/pull/1225))
  - Simplify `match_class!` syntax + implementation ([#1237](https://github.com/godot-rust/gdext/pull/1237))
  - Support `mut` bindings in `match_class!` ([#1242](https://github.com/godot-rust/gdext/pull/1242))
- Type-safe `call_deferred` alternative ([#1204](https://github.com/godot-rust/gdext/pull/1204))
- Access all enum/bitfield values programmatically ([#1232](https://github.com/godot-rust/gdext/pull/1232))

### 🧹 Quality of life

- Start phasing out `dict!` macro in favor of `vdict!` ([#1234](https://github.com/godot-rust/gdext/pull/1234))
- `RawGd` casting is now simpler and safer ([#1226](https://github.com/godot-rust/gdext/pull/1226))
- Improve `Debug` impl for objects ([#1227](https://github.com/godot-rust/gdext/pull/1227))
- Verify that panic messages support UTF-8 ([#1229](https://github.com/godot-rust/gdext/pull/1229))
- Clarify lifetimes: `GdRef<T>` -> `GdRef<'_, T>` ([#1238](https://github.com/godot-rust/gdext/pull/1238))

### 📚 Documentation

- Update editor plugin docs ([#1233](https://github.com/godot-rust/gdext/pull/1233))
- Document how to use custom getters/setters with the `OnEditor<T>` ([#1240](https://github.com/godot-rust/gdext/pull/1240))
- Clarify `Export` semantics for objects ([#1244](https://github.com/godot-rust/gdext/pull/1244))


## [v0.3.2](https://docs.rs/godot/0.3.2)

_3 July 2025_

### 🌻 Features

- `vslice![a, b]` for variant slices ([#1191](https://github.com/godot-rust/gdext/pull/1191))
- Disconnection of type-safe signals ([#1198](https://github.com/godot-rust/gdext/pull/1198))
- Callables linked to objects; let Godot auto-disconnect signals ([#1223](https://github.com/godot-rust/gdext/pull/1223))

### 🧹 Quality of life

- Implement `Debug` for `OnEditor` ([#1189](https://github.com/godot-rust/gdext/pull/1189))
- `Color`: const constructors, add `ALL_GODOT_COLORS` constant ([#1194](https://github.com/godot-rust/gdext/pull/1194))
- Deny manual `init()` if `#[class(init|no_init)]` is present ([#1196](https://github.com/godot-rust/gdext/pull/1196))
- Relaxed Variant conversions ([#1201](https://github.com/godot-rust/gdext/pull/1201))
- Allow custom types to be passed as `impl AsArg<T>` ([#1193](https://github.com/godot-rust/gdext/pull/1193))
- Verify that marshalling errors cause failed *GDScript* function ([#1203](https://github.com/godot-rust/gdext/pull/1203))
- Inline most string interpolations (`cargo +nightly clippy --fix --workspace`) ([#1206](https://github.com/godot-rust/gdext/pull/1206))
- Work around breaking change in GDExtension API (`VisualShader` class) ([#1210](https://github.com/godot-rust/gdext/pull/1210))
- Allow `clippy::uninlined_format_args` (Rust 1.88) ([#1222](https://github.com/godot-rust/gdext/pull/1222))

### 🛠️ Bugfixes

- Fix inaccurate `Color` constants ([#1195](https://github.com/godot-rust/gdext/pull/1195))
- Make hot-reload work with `#[class(no_init)]` ([#1197](https://github.com/godot-rust/gdext/pull/1197))
- Wasm registration fn names now based on crate name + index ([#1205](https://github.com/godot-rust/gdext/pull/1205))
- Fixed bug causing `ConnectHandle::is_connected()` to sometimes panic ([#1212](https://github.com/godot-rust/gdext/pull/1212))


## [v0.3.1](https://docs.rs/godot/0.3.1)

_5 June 2025_

### 🌻 Features

- Support `@export_file`, `@export_dir` etc. for `Array<GString>` and `PackedStringArray` ([#1166](https://github.com/godot-rust/gdext/pull/1166))
- Support `@export_storage` attribute ([#1183](https://github.com/godot-rust/gdext/pull/1183))
- Implement `XformInv<...>` for `Transform2D`, `Transform3D`, `Basis` ([#1082](https://github.com/godot-rust/gdext/pull/1082))
- Implement `GString` concatenation operator ([#1117](https://github.com/godot-rust/gdext/pull/1117))
- Codegen from user-provided JSON via `api-custom-json` feature ([#1124](https://github.com/godot-rust/gdext/pull/1124))
- String formatting: support padding, alignment and precision ([#1161](https://github.com/godot-rust/gdext/pull/1161))

### 📈 Performance

- Switch from `Option` to `ManuallyDrop` for blocking guard inner type ([#1176](https://github.com/godot-rust/gdext/pull/1176))

### 🛠️ Bugfixes

- Release CI: fix doc post-processing, add integration tests ([#1187](https://github.com/godot-rust/gdext/pull/1187))

### 📚 Documentation

- Improve `#[var]` + `#[export]` docs ([#1188](https://github.com/godot-rust/gdext/pull/1188))
- Clarify `Node::duplicate()` semantics on `#[var]` and `#[export]` fields ([#1141](https://github.com/godot-rust/gdext/pull/1141))


## v0.3.0

_31 May 2025_

See [devlog article](https://godot-rust.github.io/dev/may-2025-update) for highlights, and [migration guide](https://godot-rust.github.io/book/migrate/v0.3.html) to update.

### 🌻 Features

- Godot 4.4 support ([#1065](https://github.com/godot-rust/gdext/pull/1065))
- Type-safe signals
  - 🌊 User-defined signals ([#1000](https://github.com/godot-rust/gdext/pull/1000))
  - Explicit signal visibility ([#1075](https://github.com/godot-rust/gdext/pull/1075))
  - Type-safe signals for engine classes ([#1111](https://github.com/godot-rust/gdext/pull/1111))
  - Inherited typed signals ([#1134](https://github.com/godot-rust/gdext/pull/1134))
  - `emit()` now available on inherited symbols + smaller cleanups ([#1135](https://github.com/godot-rust/gdext/pull/1135))
  - User classes expose typed-signal API even without `#[signal]` ([#1146](https://github.com/godot-rust/gdext/pull/1146))
  - Generated `emit()` functions now take `impl AsArg<T>` ([#1150](https://github.com/godot-rust/gdext/pull/1150))
  - Simplify `connect*` usage ([#1152](https://github.com/godot-rust/gdext/pull/1152))
  - Clean up `ConnectBuilder` and some other signal APIs ([#1171](https://github.com/godot-rust/gdext/pull/1171))
  - `ConnectBuilder::connect_*_gd()` takes `Gd` instead of `&mut Gd` ([#1175](https://github.com/godot-rust/gdext/pull/1175))
  - Replace macro approach with indirect trait ([#1179](https://github.com/godot-rust/gdext/pull/1179))
- Async/await
  - Async Signals ([#1043](https://github.com/godot-rust/gdext/pull/1043))
  - Allow `Gd<T>` to be passed as a parameter in async signals ([#1091](https://github.com/godot-rust/gdext/pull/1091))
  - Prevent `signal_future_send_arg_no_panic` test from panicking ([#1137](https://github.com/godot-rust/gdext/pull/1137))
  - Itest runner must call `on_finished` deferred ([#1095](https://github.com/godot-rust/gdext/pull/1095))
  - Impl `DynamicSend` for `Array<T>` ([#1122](https://github.com/godot-rust/gdext/pull/1122))
- Registration
  - 🌊 Add `OnEditor<T>`, remove `impl<T> Export for Gd<T>` and `DynGd<T, D>` ([#1051](https://github.com/godot-rust/gdext/pull/1051), [#1079](https://github.com/godot-rust/gdext/pull/1079))
  - Add `OnReady::from_loaded()` + `#[init(load = "PATH")]` ([#1083](https://github.com/godot-rust/gdext/pull/1083))
  - Add support for `@experimental` and `@deprecated` attributes for user-generated docs ([#1114](https://github.com/godot-rust/gdext/pull/1114))
- Builtin types
  - Callables to builtin methods; `Array::bsearch_by, sort_unstable_by` ([#1064](https://github.com/godot-rust/gdext/pull/1064))
  - `GString`, `StringName`: add conversions from bytes and C-strings ([#1062](https://github.com/godot-rust/gdext/pull/1062))
  - `Array`, `Dictionary`: add `into_read_only()` + `is_read_only()` ([#1096](https://github.com/godot-rust/gdext/pull/1096))
- Interface traits
  - Virtual methods can become optional/required/removed in derived classes ([#1136](https://github.com/godot-rust/gdext/pull/1136))
  - 🌊 Final and non-instantiable classes ([#1162](https://github.com/godot-rust/gdext/pull/1162))
  - 🌊 Final classes no longer have a `I*` interface trait ([#1182](https://github.com/godot-rust/gdext/pull/1182))
- Support `f32` directly in `process` and `physics_process` ([#1110](https://github.com/godot-rust/gdext/pull/1110))

### 📈 Performance

- Reduce number of classes in minimal codegen ([#1099](https://github.com/godot-rust/gdext/pull/1099))
- Decrease `CallError` size from 176 to 8 bytes ([#1167](https://github.com/godot-rust/gdext/pull/1167))

### 🧹 Quality of life

- Usability
  - Propagate panics in object constructors to `Gd::from_init_fn()`, `new_gd()`, `new_alloc()` ([#1140](https://github.com/godot-rust/gdext/pull/1140))
  - 🌊 Correct `ConnectFlags` classification (enum -> bitfield) ([#1002](https://github.com/godot-rust/gdext/pull/1002))
  - Bitfields now have `|=` operator ([#1097](https://github.com/godot-rust/gdext/pull/1097))
  - Panic handling: thread safety; set hook once and not repeatedly ([#1037](https://github.com/godot-rust/gdext/pull/1037))
  - Add diagnostic hints for missing `ToGodot`/`FromGodot` traits ([#1084](https://github.com/godot-rust/gdext/pull/1084))
  - `bind/bind_mut` borrow errors now print previous stacktrace ([#1094](https://github.com/godot-rust/gdext/pull/1094))
  - Make CollisionShapes `...debug_color` methods available in Release builds ([#1149](https://github.com/godot-rust/gdext/pull/1149))
  - 🌊 Add `_rawptr` suffix to all unsafe virtual functions ([#1174](https://github.com/godot-rust/gdext/pull/1174))
  - 🌊 Remove deprecated symbols for v0.3 ([#1160](https://github.com/godot-rust/gdext/pull/1160))
- Refactoring
  - Experiment with splitting up signature differently ([#1042](https://github.com/godot-rust/gdext/pull/1042))
  - XML doc generation: code cleanup ([#1077](https://github.com/godot-rust/gdext/pull/1077))
  - `GodotFfi::variant_type` can be constant ([#1090](https://github.com/godot-rust/gdext/pull/1090))
  - Refactor parsing of `#[godot_api]` inner attributes ([#1154](https://github.com/godot-rust/gdext/pull/1154))
  - Adjust test to account for `Node::set_name()` change (`String` -> `StringName`) ([#1153](https://github.com/godot-rust/gdext/pull/1153))
- Dependencies, project structure, tooling
  - Move examples out of repository ([#1085](https://github.com/godot-rust/gdext/pull/1085))
  - Remove paste, simplify plugin macros ([#1069](https://github.com/godot-rust/gdext/pull/1069))
  - 🌊 Bump MSRV from 1.80 to 1.87 ([#1076](https://github.com/godot-rust/gdext/pull/1076), [#1184](https://github.com/godot-rust/gdext/pull/1184))
  - Validate that `api-custom` is run for Godot Debug binary ([#1071](https://github.com/godot-rust/gdext/pull/1071))
  - Centralize + update dependencies in workspace `Cargo.toml` ([#1127](https://github.com/godot-rust/gdext/pull/1127))
  - 🌊 Reduce number of classes in minimal codegen ([#1099](https://github.com/godot-rust/gdext/pull/1099))
  - Post `Rust 1.86` update: apply clippy lints ([#1115](https://github.com/godot-rust/gdext/pull/1115))
  - Move `itest` default feature `codegen-full` into build script ([#1100](https://github.com/godot-rust/gdext/pull/1100))

### 🛠️ Bugfixes

- WebAssembly
  - Wasm threading fixes ([#1093](https://github.com/godot-rust/gdext/pull/1093))
  - Remove `gensym`, fix Wasm class registration ([#1092](https://github.com/godot-rust/gdext/pull/1092))
  - Undo Wasm threading fix on panic context tracking ([#1107](https://github.com/godot-rust/gdext/pull/1107))
- Editor integration
  - Fix editor docs not generating when class itself is undocumented ([#1089](https://github.com/godot-rust/gdext/pull/1089))
  - Fix crash related to adding `EditorPlugin` to the editor before all the classes are registered ([#1138](https://github.com/godot-rust/gdext/pull/1138))
- Misc
  - 🌊 `Callable::from_local_static()` now requires Godot 4.4+ ([#1029](https://github.com/godot-rust/gdext/pull/1029))
  - Masked enums can now be constructed from integers ([#1106](https://github.com/godot-rust/gdext/pull/1106))
  - Properly set `GDExtensionBool` is_valid to true in `to_string` function ([#1145](https://github.com/godot-rust/gdext/pull/1145))
  - Virtual dispatch: fix incorrect matching of renamed methods ([#1173](https://github.com/godot-rust/gdext/pull/1173))
- Tooling workarounds
  - Release job in minimal CI; temporarily work around Godot blocker ([#1143](https://github.com/godot-rust/gdext/pull/1143))
  - Work around `OpenXR*` APIs wrongly exposed in release; wider release checks in CI ([#1070](https://github.com/godot-rust/gdext/pull/1070))
  - Work around `ResourceDeepDuplicateMode` wrongly marked as an global enum in `extension_api.json` ([#1180](https://github.com/godot-rust/gdext/pull/1180))

### 📚 Documentation

- Move builtin API design to `__docs` module ([#1063](https://github.com/godot-rust/gdext/pull/1063))
- Regression test for checking if `RustCallable` is connected to signal ([#1068](https://github.com/godot-rust/gdext/pull/1068))
- Describe semantics of `base_mut()` in docs ([#1103](https://github.com/godot-rust/gdext/pull/1103))
- Document `DynGd<_, D>` type inference ([#1142](https://github.com/godot-rust/gdext/pull/1142))
- `#[derive(GodotClass)]`, `#[godot_api]` docs: replace table of contents with sidebar ([#1155](https://github.com/godot-rust/gdext/pull/1155))
- Add `rustc-args` to `package.metadata.docs.rs` ([#1169](https://github.com/godot-rust/gdext/pull/1169))
- Document runtime class in editor requirement ([#1168](https://github.com/godot-rust/gdext/pull/1168))
- Improve signal/async docs ([#1184](https://github.com/godot-rust/gdext/pull/1184))


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
