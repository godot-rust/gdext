![logo.png](misc/assets/godot-rust-ferris.png)

# Rust bindings for Godot 4

_**[Website]** | **[Book][book]** | **[API Docs]** | [Discord] | [BlueSky] | [Mastodon] | [Twitter] | [Sponsor]_

**godot-rust** is a library to integrate the Rust language with Godot 4.

[Godot] is an open-source game engine, focusing on a productive and batteries-included 2D and 3D experience.  
Its _GDExtension_ API allows integrating third-party languages and libraries.


## Philosophy

The Rust binding is an alternative to GDScript, with a focus on type safety, scalability and performance.

The primary goal of godot-rust is to provide a [**pragmatic Rust API**][philosophy] for game developers.

Recurring workflows should be simple and require minimal boilerplate. APIs are designed to be safe and idiomatic Rust wherever possible.
Due to interacting with Godot as a C++ engine, we sometimes follow unconventional approaches to provide a good user experience.


## Motivating example

The following Rust snippet registers a Godot class `Player`, showcasing features such as inheritance, field initialization and signals.

```rust
use godot::classes::{ISprite2D, ProgressBar, Sprite2D};
use godot::prelude::*;

// Declare the Player class inheriting Sprite2D.
#[derive(GodotClass)]
#[class(init, base=Sprite2D)] // Automatic initialization, no manual init() needed.
struct Player {
    // Inheritance via composition: access to Sprite2D methods.
    base: Base<Sprite2D>,

    // #[class(init)] above allows attribute-initialization of fields.
    #[init(val = 100)]
    hitpoints: i32,

    // Access to a child node, auto-initialized when _ready() is called.
    #[init(node = "Ui/HealthBar")] // <- Path to the node in the scene tree.
    health_bar: OnReady<Gd<ProgressBar>>,
}

// Implement Godot's virtual methods via predefined trait.
#[godot_api]
impl ISprite2D for Player {
    // Override the `_ready` method.
    fn ready(&mut self) {
        godot_print!("Player ready!");

        // Health bar is already initialized and straightforward to access.
        self.health_bar.set_max(self.hitpoints as f64);
        self.health_bar.set_value(self.hitpoints as f64);

        // Connect type-safe signal: print whenever the health bar is updated.
        self.health_bar.signals().value_changed().connect(|hp| {
            godot_print!("Health changed to: {hp}");
        });
    }
}

// Implement custom methods that can be called from GDScript.
#[godot_api]
impl Player {
    #[func]
    fn take_damage(&mut self, damage: i32) {
        self.hitpoints -= damage;
        godot_print!("Player hit! HP left: {}", self.hitpoints);

        // Update health bar.
        self.health_bar.set_value(self.hitpoints as f64);

        // Call Node methods on self, via mutable base access.
        if self.hitpoints <= 0 {
            self.base_mut().queue_free();
        }
    }
}
```


## Development status

The library has evolved a lot since 2023 and is now in a usable state for projects such as games, editor plugins, tools and other applications
based on Godot. See [ecosystem] to get an idea of what users have built with godot-rust.

Keep in mind that we occasionally introduce breaking changes, motivated by improved user experience or upstream changes. These are usually
minor and accompanied by migration guides. Our [crates.io releases][crates-io] adhere to SemVer, but lag a bit behind the `master` branch.
See also [API stability] in the book.

The vast majority of Godot APIs have been mapped to Rust. The current focus lies on a more natural Rust experience and enable more design
patterns that come in handy for day-to-day game development. To counter bugs, we use an elaborate CI suite including clippy, unit tests,
engine integration tests and memory sanitizers. Even hot-reload is tested!

At the moment, there is experimental support for [Wasm], [Android] and [iOS], but documentation and tooling is still lacking.
Contributions are very welcome!


## Getting started

The best place to start is [the godot-rust book][book]. Use it in conjunction with our [API Docs].  
We also provide practical examples and small games in the [demo-projects] repository.

If you need help, join our [Discord] server and ask in the `#help` channel!


## License

We use the [Mozilla Public License 2.0][mpl]. MPL tries to find a balance between permissive (MIT, Apache, Zlib) and copyleft licenses (GPL, LGPL).

The license provides a lot of freedom: you can use the library commercially and keep your own code closed-source,
i.e. game development is not restricted. The main condition is that if you change godot-rust _itself_, you need to make
those changes available (and only those, no surrounding code).


## Contributing

Contributions are very welcome! If you want to help out, see [`Contributing.md`](Contributing.md) for some pointers on getting started.

[API Docs]: https://godot-rust.github.io/docs/gdext
[API stability]: https://godot-rust.github.io/book/toolchain/compatibility.html#rust-api-stability
[Android]: https://github.com/godot-rust/gdext/issues/470
[Discord]: https://discord.gg/aKUCJ8rJsc
[Godot]: https://godotengine.org
[BlueSky]: https://bsky.app/profile/godot-rust.bsky.social
[Mastodon]: https://mastodon.gamedev.place/@GodotRust
[Sponsor]: https://github.com/sponsors/Bromeon
[Twitter]: https://twitter.com/GodotRust
[WASM]: https://godot-rust.github.io/book/toolchain/export-web.html
[Website]: https://godot-rust.github.io
[`gdnative`]: https://github.com/godot-rust/gdnative
[book]: https://godot-rust.github.io/book
[ecosystem]: https://godot-rust.github.io/book/ecosystem
[demo-projects]: https://github.com/godot-rust/demo-projects
[iOS]: https://github.com/godot-rust/gdext/issues/498
[mpl]: https://www.mozilla.org/en-US/MPL
[philosophy]: https://godot-rust.github.io/book/contribute/philosophy.html
[crates-io]: https://crates.io/crates/godot
