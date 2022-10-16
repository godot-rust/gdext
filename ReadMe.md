# Rust bindings for GDExtension

This is an early-stage library to bind the **Rust** language to **Godot 4**.

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

So we do not recommend building a larger project in GDExtension-Rust yet.
However, the library can serve as a playground for experimenting or smaller projects, such as game jams. 


## Usage

We currently only have a GitHub version, crates.io releases are planned once more of the foundation is ready.  
In your Cargo.toml, add:

```toml
godot = { git = "https://github.com/godot-rust/gdextension", branch = "master" }
```
To get the latest changes, you can regularly run a `cargo update` (possibly breaking). Keep your `Cargo.lock` file under version control, so that it's easy to revert updates.

If you need help, join our [Discord] server and ask in the `#help` channel!


## License

We use the [Mozilla Public License 2.0][mpl]. MPL tries to find a balance between permissive (MIT, Apache, Zlib) and copyleft licenses (GPL, LGPL).

The license provides a lot of freedom: you can use the library commercially and keep your own code closed-source,
i.e. game development is not restricted. The main condition is that if you change godot-rust _itself_, you need to make 
those changes available (and only those, no surrounding code).


## Contributing

At this stage, we appreciate if users experiment with the library, use it in small projects and report issues and bugs they encounter.

If you plan to make bigger contributions, make sure to discuss them in a GitHub issue first. Since the library is evolving quickly, this
avoids that multiple people work on the same thing or implement features in a way that doesn't work with other parts. Also don't hesitate
to talk to the developers in the `#gdext-dev` channel on [Discord]!


[Godot]: https://godotengine.org
[`gdnative`]: https://github.com/godot-rust/godot-rust
[mpl]: https://www.mozilla.org/en-US/MPL/
[Discord]: https://discord.gg/aKUCJ8rJsc