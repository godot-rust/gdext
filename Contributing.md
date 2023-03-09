# Contributing to `gdext`

At this stage, we appreciate if users experiment with the library, use it in small projects and report issues and bugs they encounter.

If you plan to make bigger contributions, make sure to discuss them in a [GitHub issue] first. Since the library is evolving quickly, this avoids that multiple people work on the same thing or implement features in a way that doesn't work with other parts. Also don't hesitate to talk to the developers in the `#contrib-gdext` channel on [Discord]!

## Check script

The script `check.sh` in the project root can be used to mimic a CI run locally. It's useful to run this before you commit, push or create a pull request:

```
$ check.sh
```

At the time of writing, this will run formatting, clippy, unit tests and integration tests. More checks may be added in the future. Run `./check.sh --help` to see all available options.

If you like, you can set this as a pre-commit hook in your local clone of the repository:

```
$ ln -sf check.sh .git/hooks/pre-commit
```

## Unit tests

Because most of `gdext` interacts with the Godot engine, which is not available from the test executable, unit tests (using `cargo test` and the `#[test]` attribute) are pretty limited in scope.

Because additional flags might be needed, the preferred way to run unit tests is through the `check.sh` script:

```
$ ./check.sh test
```

## Integration tests

The `itest/` directory contains a suite of integration tests that actually exercise `gdext` from within Godot.

The `itest/rust` directory is a Rust `cdylib` library project that can be loaded as a GDExtension in Godot, with an entry point for running integration tests. The `itest/godot` directory contains the Godot project that loads this library and invokes the test suite.

You can run the integration tests like this:

```
$ ./check.sh itest
```

Just like when compiling the crate, the `GODOT4_BIN` environment variable can be used to supply the path and filename of your Godot executable.

## Formatting

`rustfmt` is used to format code. `check.sh` only warns about formatting issues, but does not fix them. To do that, run:

```
$ cargo fmt
```

## Clippy

`clippy` is used for additional lint warnings not implemented in `rustc`. This, too, is best run through `check.sh`:

```
$ check.sh clippy
```

## Real

Certain types in Godot use either a single or double-precision float internally, such as `Vector2`. When using these types we 
use the `real` type instead of choosing either `f32` or `f64`. Thus our code is portable between Godot binaries compiled with
`precision=single` or `precision=double`.

To run the testing suite with `double-precision` enabled you may add `--double` to a `check.sh` invocation:
```
$ check.sh --double
```

[GitHub issue]: https://github.com/godot-rust/gdext/issues
[Discord]: https://discord.gg/aKUCJ8rJsc
