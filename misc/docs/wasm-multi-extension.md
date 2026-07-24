# Multiple godot-rust extensions on Wasm

Upstream issue: <https://github.com/godot-rust/gdext/issues/968>

## Symptom
A Godot project loads two or more GDExtensions that are built with godot-rust. Desktop works. Web export panics during load:

```
insert_class_name() called for already-existing string: RefCounted
```

or `already initialized` from `BindingStorage::initialize`. Removing one of the extensions makes it work again. The extensions do not need to
depend on each other -- two unrelated third-party extensions are enough.

## Cause
On Wasm/Emscripten, extensions are linked as *side modules*. A symbol with external linkage and default visibility becomes a `GOT.mem` import,
and Emscripten's dynamic linker resolves all such imports with the same name to one address:

```wasm
(import "GOT.mem" "_RNvNtNtCsiZxTL9DORka_10godot_core4meta8class_id14CLASS_ID_CACHE" (global (;22;) (mut i32)))
```

Both extensions then use the same `CLASS_ID_CACHE`, binding storage, string cache and registries. State is corrupted, and it is undefined
behavior -- the panic is godot-rust noticing, not the failure itself.

Two extensions collide only when their mangled symbols match, i.e. same godot-rust version *and* same crate disambiguator (features, cfgs).
Different feature sets produce different symbols and no collision at all.

## Why only Wasm
rustc emits the same LLVM IR for both targets: the static is `external` + `default`. On ELF, the symbol ends up `LOCAL` in the finished
`.so`, so each shared library gets its own copy. On Wasm it stays interposable. Neither Godot nor the Emscripten runtime is at fault.

## Current state
godot-rust only *diagnoses* this. `sys::MULTI_EXTENSION_HINT` (see `godot-ffi/src/multi_extension.rs`) is appended to the three panics that
can be caused by sharing, naming the cause and the user-side workaround. Empty outside Wasm. The issue is not fixed.

## Proposed fix
Ordered by cost. A is cheap to test and would need nothing from users; B is the fallback that is known to work; C is required either way.

### A. Link-time flag, injected by godot-rust (unverified)
`wasm-ld` decides `GOT.mem` entries at link time, and `godot`'s `build.rs` can emit `cargo::rustc-link-arg-cdylib=...` for the final link.
If some flag (`--no-export-dynamic`, a `-Bsymbolic` equivalent, `-sSIDE_MODULE=1` instead of `=2`) makes data references bind
module-internally, this is a one-line fix, no nightly, no user action. Unknown whether such a flag exists for Wasm *data* symbols --
visibility may already be baked into the object files, in which case a link-time flag cannot help. Worth an experiment before committing to B.

### B. Internal linkage by construction
Move every shared static into a non-generic `#[inline(never)]` accessor:

```rust
fn class_id_cache() -> &'static Global<ClassIdCache> {
    static CACHE: Global<ClassIdCache> = Global::default();
    &CACHE
}
```

A static declared in a function body gets `internal` linkage and emits no `GOT.mem` import, so each side module keeps its own copy -- same
behavior as desktop. Works on stable, needs no user cooperation.

Scope: roughly 35 statics across `godot-core` and `godot-ffi`, plus whatever `godot-codegen` emits (method tables), which needs the same
treatment in the generator.

Constraint: generic code must never touch such a static directly. A generic function is instantiated in the *user's* crate, which forces the
static back to `external`. Generic code calls the accessor instead.

### C. CI guard
Without a guard, one new `static` silently regresses this and nobody notices until the next bug report. CI already installs emsdk
(`.github/workflows/full-ci.yml`), so: build a `cdylib` for `wasm32-unknown-emscripten`, run `wasm-objdump -x`, fail if any `GOT.mem` import
matches `_RNv.*godot`. This is what turns "fixed once" into "stays fixed".

## Why not just require `-Zdefault-visibility=hidden`
It is the documented user-side workaround and it does work. It is not a fix:

- **A dependency cannot set it.** It is a compile flag over the whole build, not something `godot`'s `build.rs` can inject. Every extension
  author must opt in, by hand, forever.
- **It is cross-project.** An extension published to the Asset Library as a prebuilt `.wasm` is out of the author's control -- if a game
  combines it with another non-compliant extension, the game breaks, and neither author can fix it from their side.
- **It cannot be validated.** There is no `cfg` for it and no way for godot-core to check how it was compiled. A runtime probe does not work
  either (see below), so godot-rust can neither enforce nor detect compliance -- only guess after the fact.
- **Nightly-only and unstable.** Wasm builds already need nightly for `-Zbuild-std`, so this is the weakest point, but an unstable flag can
  change or go away.
- **It is global.** It hides symbols in every crate of the build. Whether the `gdext_rust_init` entry symbol survives this needs verifying;
  users report working setups, so presumably yes, but it is not something godot-rust would want to depend on silently.

## What doesn't work

**Runtime probe comparing an exported counter against a module-local one.** The probe symbol must be `#[no_mangle]` to be shared, so it
collides *unconditionally* -- while the actual mangled statics collide only on matching disambiguator. Two extensions with different feature
sets keep separate globals but would still be reported, aborting a load that was fine. The probe measures something other than the bug. This
is why detection is attached to the existing asserts, which fire only on real sharing.

**Keying globals per extension** (a runtime key in `Global<T>`, so shared statics still give each extension its own slot). Requires knowing
which extension is currently executing at arbitrary call sites, which is only knowable at entry points. Invasive, and costly on hot paths
like the class-ID cache.

**Distinguishing extensions by entry symbol** (`#[gdextension(entry_symbol = ...)]`). Reported in the issue as ineffective, as expected: the
entry point name has no bearing on how `godot-core`'s statics are mangled or resolved.

**A `#[gdextension(dependency)]` attribute** that skips init for non-primary extensions. Proposed early in the issue, before the cause was
known. The extensions involved need not depend on each other, and each one legitimately needs its own initialized state; the problem is
symbol resolution, not initialization order.
