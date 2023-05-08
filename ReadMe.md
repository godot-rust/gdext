![logo.png](assets/gdext-ferris.png)

# Rust bindings for GDExtension

_**[Website]** | **[API Docs]** | [Discord] | [Mastodon] | [Twitter]_

**gdext** is an early-stage library to bind the **Rust** language to **Godot 4**.

[Godot] is an open-source game engine, whose upcoming version 4.0 brings several improvements.
Its _GDExtension_ API allows integrating third-party languages and libraries.

> **Note**: if you are looking for a Rust binding for GDNative (Godot 3), checkout [`gdnative`].

> **Warning**: this library is experimental and rapidly evolving. In particular, this means:
> * Lots of bugs. A lot of the scaffolding is still being ironed out. 
>   There are known safety issues, possible undefined behavior as well as other potential problems.
> * Lots of missing features. The priority is to get basic interactions working;
>   as such, edge case APIs are deliberately neglected at this stage.
> * No stability guarantees. APIs will break frequently (for releases, we try to take SemVer seriously though).
>   Resolving the above two points has currently more weight than a stable API.

We do not recommend building a larger project in gdext yet.
However, the library can serve as a playground for experimenting.

To get an overview of currently supported features, consult [#24](https://github.com/godot-rust/gdext/issues/24).  
At this point, there is **no** support for Android, iOS or WASM. Contributions are very welcome!


## Getting started

An elaborate tutorial is available [in the book] (still under construction), here is the short version.

To find a version of Godot 4, the library expects either an executable of name `godot4` in the PATH, or an environment variable `GODOT4_BIN`
containing the path to the executable (including filename).
We currently only have a GitHub version, crates.io releases are planned once more of the foundation is ready.

In your Cargo.toml, add:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
godot = { git = "https://github.com/godot-rust/gdext", branch = "master" }
```
To get the latest changes, you can regularly run a `cargo update` (possibly breaking). Keep your `Cargo.lock` file under version control, so that it's easy to revert updates.

To register the GDExtension library with Godot, you need to create two files relative to your Godot project folder:

1. First, add `res://MyExt.gdextension`, which is the equivalent of `.gdnlib` for GDNative.

   The `[configuration]` section should be copied as-is.  
   The `[libraries]` section should be updated to match the paths of your dynamic Rust libraries.
   `{my-ext}` can be replaced with the name of your crate.
   ```ini
   [configuration]
   entry_symbol = "gdext_rust_init"
   
   [libraries]
   linux.debug.x86_64 = "res://../rust/target/debug/lib{my-ext}.so"
   linux.release.x86_64 = "res://../rust/target/release/lib{my-ext}.so"
   windows.debug.x86_64 = "res://../rust/target/debug/{my-ext}.dll"
   windows.release.x86_64 = "res://../rust/target/release/{my-ext}.dll"
   macos.debug = "res://../rust/target/debug/{my-ext}.dylib"
   macos.release = "res://../rust/target/release/{my-ext}.dylib"
   macos.debug.arm64 = "res://../rust/target/debug/{my-ext}.dylib"
   macos.release.arm64 = "res://../rust/target/release/{my-ext}.dylib"
   ```
   > **Note**: for exporting your project, you'll need to use paths inside `res://`

   > **Note**: If you specify your cargo compilation target via the `--target` flag or a `.cargo/config.toml` file, the rust library will be placed in a path name that includes target architecture, and the `.gdextension` library paths will need to match. E.g. for M1 Macs (`macos.debug.arm64` and `macos.release.arm64`) the path would be `"res://../rust/target/aarch64-apple-darwin/debug/{my-ext}.dylib"`

2. A second file `res://.godot/extension_list.cfg` should be generated once you open the Godot editor for the first time.
   If not, you can also manually create it, simply containing the Godot path to your `.gdextension` file:
   ```
   res://MyExt.gdextension
   ```

### Examples

We highly recommend to have a look at a working example in the `examples/dodge-the-creeps` directory.
This integrates a small game with Godot and has all the necessary steps set up.

API documentation can be generated locally using `./check.sh doc` (use `dok` instead of `doc` to open the page in the browser).

If you need help, join our [Discord] server and ask in the `#help-gdext` channel!


## License

We use the [Mozilla Public License 2.0][mpl]. MPL tries to find a balance between permissive (MIT, Apache, Zlib) and copyleft licenses (GPL, LGPL).

The license provides a lot of freedom: you can use the library commercially and keep your own code closed-source,
i.e. game development is not restricted. The main condition is that if you change godot-rust _itself_, you need to make
those changes available (and only those, no surrounding code).


## Contributing

Contributions are very welcome! If you want to help out, see [`Contributing.md`](Contributing.md) for some pointers on getting started!

[Godot]: https://godotengine.org
[`gdnative`]: https://github.com/godot-rust/gdnative
[mpl]: https://www.mozilla.org/en-US/MPL
[Website]: https://godot-rust.github.io
[API Docs]: https://godot-rust.github.io/docs/gdext
[Discord]: https://discord.gg/aKUCJ8rJsc
[Mastodon]: https://mastodon.gamedev.place/@GodotRust
[Twitter]: https://twitter.com/GodotRust
[in the book]: https://godot-rust.github.io/book/gdext/intro