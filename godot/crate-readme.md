![logo.png](https://github.com/godot-rust/assets/blob/master/gdext/banner.png?raw=true)

# Rust bindings for Godot 4

_**[Website]** | **[GitHub]** | **[Book]** | **[API Docs]** | [Discord] | [Mastodon] | [Twitter] | [Sponsor]_

The **godot** crate integrates the Rust language with Godot 4.

[Godot] is an open-source game engine, focusing on a productive and batteries-included 2D and 3D experience.  
Its _GDExtension_ API allows integrating third-party languages and libraries.


## Philosophy

The Rust binding is an alternative to GDScript, with a focus on type safety, scalability and performance.

The primary goal of this library is to provide a [**pragmatic Rust API**][philosophy] for game developers.
Recurring workflows should be simple and require minimal boilerplate. APIs are designed to be safe and idiomatic Rust wherever possible.
Due to interacting with Godot as a C++ engine, we sometimes follow unconventional approaches to provide a good user experience.


## Example

The following code snippet demonstrates writing a simple Godot class `Player` in Rust.

```rust
use godot::prelude::*;
use godot::classes::{ISprite2D, Sprite2D};

// Declare the Player class inheriting Sprite2D.
#[derive(GodotClass)]
#[class(base=Sprite2D)]
struct Player {
    // Inheritance via composition: access to Sprite2D methods.
    base: Base<Sprite2D>,

    // Other fields.
    velocity: Vector2,
    hitpoints: i32,
}

// Implement Godot's virtual methods via predefined trait.
#[godot_api]
impl ISprite2D for Player {
    // Default constructor (base object is passed in).
    fn init(base: Base<Sprite2D>) -> Self {
        Player {
            base,
            velocity: Vector2::ZERO,
            hitpoints: 100,
        }
    }

    // Override the `_ready` method.
    fn ready(&mut self) {
        godot_print!("Player ready!");
    }
}

// Implement custom methods that can be called from GDScript.
#[godot_api]
impl Player {
    #[func]
    fn take_damage(&mut self, damage: i32) {
        self.hitpoints -= damage;
        godot_print!("Player hit! HP left: {}", self.hitpoints);
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
[philosophy]: https://godot-rust.github.io/book/contribute/philosophy.html
[Sponsor]: https://github.com/sponsors/Bromeon
[Twitter]: https://twitter.com/GodotRust
[Website]: https://godot-rust.github.io
