![logo.png](https://github.com/godot-rust/assets/blob/master/gdext/banner.png?raw=true)

# Rust bindings for Godot 4

_**[Website]** | **[GitHub]** | **[Book]** | **[API Docs]**_   
_[Discord] | [BlueSky] | [Mastodon] | [Twitter] | [Sponsor]_

The **godot** crate integrates the Rust language with Godot 4.

[Godot] is an open-source game engine, focusing on a productive and batteries-included 2D and 3D experience.
Its _GDExtension_ API allows integrating third-party languages and libraries.


## Philosophy

The Rust binding is an alternative to GDScript, with a focus on type safety, scalability and performance.

The primary goal of this library is to provide a [**pragmatic Rust API**][philosophy] for game developers.
Recurring workflows should be simple and require minimal boilerplate. APIs are designed to be safe and idiomatic Rust wherever possible.
Due to interacting with Godot as a C++ engine, we sometimes follow unconventional approaches to provide a good user experience.


## Example

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


## More

For more information, check out our [Website] or [GitHub] page!


[API Docs]: https://godot-rust.github.io/docs/gdext
[Book]: https://godot-rust.github.io/book
[Discord]: https://discord.gg/aKUCJ8rJsc
[GitHub]: https://github.com/godot-rust/gdext
[Godot]: https://godotengine.org
[Mastodon]: https://mastodon.gamedev.place/@GodotRust
[BlueSky]: https://bsky.app/profile/godot-rust.bsky.social
[philosophy]: https://godot-rust.github.io/book/contribute/philosophy.html
[Sponsor]: https://github.com/sponsors/Bromeon
[Twitter]: https://twitter.com/GodotRust
[Website]: https://godot-rust.github.io
